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

use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::time::Duration;
use crossbeam_channel::Sender;
use time::OffsetDateTime;
use tracing_core::{Event, Level};
use tracing_core::span::{Attributes, Id, Record};
use crate::core::{Tracer, TracingSystem};
use crate::profiler::auto_discover::AutoDiscoveryService;
use crate::profiler::DEFAULT_PORT;
use crate::profiler::logpump::LOG_PUMP;
use crate::profiler::network_types::{Hello, HELLO_PACKET, MatchResult};
use crate::profiler::state::ProfilerState;
use crate::profiler::thread::{Command, Thread};
use crate::profiler::visitor::Visitor;
use crate::profiler::network_types::Duration as NetDuration;

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        ProfilerState::get().terminate();
    }
}

fn handle_hello(client: &mut TcpStream) -> std::io::Result<()> {
    let bytes = HELLO_PACKET.to_bytes();
    let mut block = [0; 40];
    client.write(&bytes)?;
    client.read_exact(&mut block)?;
    let packet = Hello::from_bytes(block);
    match HELLO_PACKET.matches(&packet) {
        MatchResult::SignatureMismatch => Err(Error::new(ErrorKind::Other, "protocol signature mismatch")),
        MatchResult::VersionMismatch => Err(Error::new(ErrorKind::Other, "version signature mismatch")),
        MatchResult::Ok => Ok(())
    }
}

pub struct Profiler {
    channel: Sender<Command>
}

impl Profiler {
    pub fn new(app_name: &str, crate_name: &str, crate_version: &str) -> std::io::Result<TracingSystem<Profiler>> {
        log::set_logger(&LOG_PUMP).expect("Cannot initialize profiler more than once!");
        let port = bp3d_env::get("PROFILER_PORT")
            .map(|v| v.parse().unwrap_or(DEFAULT_PORT))
            .unwrap_or(DEFAULT_PORT);
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        let listener = TcpListener::bind(addr)?;
        let service = AutoDiscoveryService::new(app_name)?;
        let exit_flag = service.get_exit_flag();
        let thread = std::thread::spawn(move || {
            service.run();
        });
        println!("Waiting for debugger to attach to {}...", port);
        //Block software until we receive a debugger connection.
        let (mut client, _) = listener.accept()?;
        exit_flag.store(true, Ordering::Relaxed);
        thread.join().unwrap();
        handle_hello(&mut client)?;
        let (sender, receiver) = ProfilerState::get().get_channel();
        let thread = std::thread::spawn(|| {
            let mut thread = Thread::new(client, receiver);
            thread.run();
        });
        sender.send(Command::Project {
            app_name: app_name.into(),
            name: crate_name.into(),
            version: crate_version.into()
        }).unwrap();
        ProfilerState::get().assign_thread(thread);
        log::set_max_level(log::LevelFilter::Trace);
        Ok(TracingSystem::with_destructor(Profiler {
            channel: sender
        }, Box::new(Guard)))
    }

    fn is_exited(&self) -> bool {
        ProfilerState::get().is_exited()
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
            });
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
            duration: NetDuration {
                seconds: duration.as_secs(),
                nano_seconds: duration.subsec_nanos()
            }
        });
    }

    fn span_destroy(&self, id: &Id) {
        self.command(Command::SpanFree(id.into_u64()));
    }

    fn max_level_hint(&self) -> Option<Level> {
        None
    }
}
