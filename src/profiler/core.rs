// Copyright (c) 2022, BlockProject 3D
//
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
//     * Redistributions of source code must retain the above copyright notice,
//       this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above copyright notice,
//       this list of conditions and the following disclaimer in the documentation
//       and/or other materials provided with the distribution.
//     * Neither the name of BlockProject 3D nor the names of its contributors
//       may be used to endorse or promote products derived from this software
//       without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use crossbeam_channel::{bounded, Sender};
use time::OffsetDateTime;
use tracing_core::{Event, Level};
use tracing_core::dispatcher::get_default;
use tracing_core::span::{Attributes, Id, Record};
use crate::core::{Tracer, TracingSystem};
use crate::profiler::thread::{Command, Thread};
use crate::profiler::visitor::Visitor;

const MAX_COMMANDS: usize = 32;
const DEFAULT_PORT: u16 = 9999;

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        get_default(|dispatcher| {
            let profiler: &Profiler = dispatcher.downcast_ref().unwrap();
            profiler.terminate();
        });
    }
}

pub struct Profiler {
    channel: Sender<Command>,
    exited: AtomicBool,
    thread: Mutex<Option<JoinHandle<()>>>
}

impl Profiler {
    pub fn new() -> std::io::Result<TracingSystem<Profiler>> {
        let port = std::env::var("PROFILER_PORT")
            .map(|v| v.parse().unwrap_or(DEFAULT_PORT))
            .unwrap_or(DEFAULT_PORT);
        println!("Waiting for debugger to attach to {}...", port);
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        let listener = TcpListener::bind(addr)?;
        //Block software until we receive a debugger connection.
        let (client, _) = listener.accept()?;
        let (sender, receiver) = bounded(MAX_COMMANDS);
        let thread = std::thread::spawn(|| {
            let mut thread = Thread::new(client, receiver);
            thread.run();
        });
        Ok(TracingSystem::with_destructor(Profiler {
            channel: sender,
            exited: AtomicBool::new(false),
            thread: Mutex::new(Some(thread))
        }, Box::new(Guard)))
    }

    //Terminate debugging session.
    pub fn terminate(&self) {
        if self.is_exited() {
            return;
        }
        self.channel.send(Command::Terminate).unwrap();
        let thread = {
            let mut lock = self.thread.lock().unwrap();
            lock.take()
        };
        if let Some(thread) = thread {
            thread.join().unwrap();
        }
        self.exited.store(true, Ordering::Relaxed);
    }

    fn is_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    fn command(&self, cmd: Command) {
        if !self.is_exited() {
            self.channel.send(cmd).unwrap();
        }
    }
}

impl Tracer for Profiler {
    fn enabled(&self) -> bool {
        !self.is_exited()
    }

    fn span_create(&self, id: &Id, new: bool, parent: Option<Id>, span: &Attributes) {
        if new {
            self.command(Command::SpanAlloc {
                metadata: span.metadata(),
                id: id.into_u64()
            })
        }
        let mut visitor = Visitor::new();
        span.record(&mut visitor);
        let (message, value_set) = visitor.into_inner();
        self.command(Command::SpanInit {
            span: id.into_u64(),
            value_set,
            message,
            parent: parent.map(|v| v.into_u64())
        });
    }

    fn span_values(&self, id: &Id, values: &Record) {
        let mut visitor = Visitor::new();
        values.record(&mut visitor);
        let (message, value_set) = visitor.into_inner();
        self.command(Command::SpanValues {
            span: id.into_u64(),
            message,
            value_set
        });
    }

    fn span_follows_from(&self, id: &Id, follows: &Id) {
        self.command(Command::SpanFollows {
            span: id.into_u64(),
            follows: follows.into_u64()
        });
    }

    fn event(&self, parent: Option<Id>, time: OffsetDateTime, event: &Event) {
        let mut visitor = Visitor::new();
        event.record(&mut visitor);
        let (message, value_set) = visitor.into_inner();
        self.command(Command::Event(crate::profiler::thread::Event::Borrowed {
            metadata: event.metadata(),
            span: parent.map(|v| v.into_u64()),
            message,
            value_set,
            time: time.unix_timestamp()
        }));
    }

    fn span_enter(&self, id: &Id) {
        self.command(Command::SpanEnter(id.into_u64()));
    }

    fn span_exit(&self, id: &Id, duration: Duration) {
        self.command(Command::SpanExit {
            span: id.into_u64(),
            duration: duration.as_secs_f64()
        });
    }

    fn span_destroy(&self, id: Id) {
        self.command(Command::SpanFree(id.into_u64()));
    }

    fn max_level_hint(&self) -> Option<Level> {
        None
    }
}
