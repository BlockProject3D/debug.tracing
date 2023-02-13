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

use crate::core::{Tracer, TracingSystem};
use crate::profiler::logpump::LOG_PUMP;
use crate::profiler::network_types::{Hello, MatchResult, HELLO_PACKET};
use crate::profiler::state::ProfilerState;
use crate::profiler::thread::{Command, FixedBufStr, Thread};
use crate::profiler::visitor::{ChannelVisitor};
use crate::profiler::DEFAULT_PORT;
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::time::Duration;
use bp3d_fs::dirs::App;
use bp3d_logger::LogMsg;
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Level};
use crate::util::{extract_target_module, tracing_level_to_log};
use crate::visitor::FastVisitor;

struct Guard(ProfilerState);

impl Drop for Guard {
    fn drop(&mut self) {
        self.0.terminate()

    }
}

fn handle_hello(client: &mut TcpStream) -> std::io::Result<()> {
    let bytes = HELLO_PACKET.to_bytes();
    let mut block = [0; 40];
    client.write(&bytes)?;
    client.read_exact(&mut block)?;
    let packet = Hello::from_bytes(block);
    match HELLO_PACKET.matches(&packet) {
        MatchResult::SignatureMismatch => {
            Err(Error::new(ErrorKind::Other, "protocol signature mismatch"))
        }
        MatchResult::VersionMismatch => {
            Err(Error::new(ErrorKind::Other, "version signature mismatch"))
        }
        MatchResult::Ok => Ok(()),
    }
}

pub struct Profiler {
    channel: Sender<Command>
}

impl Profiler {
    pub fn new(
        app_name: &str,
        crate_name: &str,
        crate_version: &str,
    ) -> std::io::Result<TracingSystem<Profiler>> {
        log::set_logger(&LOG_PUMP).expect("Cannot initialize profiler more than once!");
        let port = bp3d_env::get("PROFILER_PORT")
            .map(|v| v.parse().unwrap_or(DEFAULT_PORT))
            .unwrap_or(DEFAULT_PORT);
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        let listener = TcpListener::bind(addr)?;
        println!("Waiting for debugger to attach to {}...", port);
        //Block software until we receive a debugger connection.
        let (mut client, _) = listener.accept()?;
        //handle_hello(&mut client)?;
        let logs = App::new(app_name).get_logs().ok().map(|v| v.to_owned());
        let (state, sender) = ProfilerState::new(|receiver| {
            let mut thread = Thread::new(client, receiver/*, breceiver*/, logs);
            thread.run();
        });
        sender.send(Command::Project {
                app_name: FixedBufStr::from_str(app_name),
                name: FixedBufStr::from_str(crate_name),
                version: FixedBufStr::from_str(crate_version),
        }).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
        Ok(TracingSystem::with_destructor(
            Profiler { channel: sender/*, bytes_channel: bsender*/ },
            Box::new(Guard(state)),
        ))
    }

    #[inline]
    fn command(&self, cmd: Command) {
        let _ = self.channel.send(cmd);
    }
}

impl Tracer for Profiler {
    fn enabled(&self) -> bool {
        true
    }

    fn span_create(&self, id: &Id, new: bool, parent: Option<Id>, span: &Attributes) {
        if new {
            self.command(Command::SpanAlloc {
                metadata: span.metadata(),
                id: id.into_u64(),
            });
        }
        self.command(Command::SpanInit {
            span: id.into_u64(),
            parent: parent.map(|v| v.into_u64()),
        });
        let mut visitor = ChannelVisitor::new(&self.channel, id.into_u64());
        span.record(&mut visitor);
    }

    fn span_values(&self, id: &Id, values: &Record) {
        let mut visitor = ChannelVisitor::new(&self.channel, id.into_u64());
        values.record(&mut visitor);
    }

    fn span_follows_from(&self, id: &Id, follows: &Id) {
        self.command(Command::SpanFollows {
            span: id.into_u64(),
            follows: follows.into_u64(),
        });
    }

    fn event(&self, parent: Option<Id>, time: DateTime<Utc>, event: &Event) {
        let (target, module) = extract_target_module(event.metadata());
        let mut msg = LogMsg::new(target, tracing_level_to_log(event.metadata().level()));
        use std::fmt::Write;
        let _ = write!(msg, "{}: ", module.unwrap_or("main"));
        let mut visitor = FastVisitor::new(&mut msg, event.metadata().name());
        event.record(&mut visitor);
        self.command(Command::Event {
            id: parent.map(|v| (v.into_u64() >> 32) as u32).unwrap_or(0),
            message: msg,
            timestamp: time.timestamp()
        });
    }

    fn span_enter(&self, id: &Id) {
        self.command(Command::SpanEnter(id.into_u64()));
    }

    fn span_exit(&self, id: &Id, duration: Duration) {
        self.command(Command::SpanExit {
            span: id.into_u64(),
            duration,
        });
    }

    fn span_destroy(&self, id: &Id) {
        self.command(Command::SpanFree(id.into_u64()));
    }

    fn max_level_hint(&self) -> Option<Level> {
        None
    }
}
