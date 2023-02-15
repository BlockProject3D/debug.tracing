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

use tokio::sync::oneshot;
use crate::core::{Tracer, TracingSystem};
use crate::profiler::logpump::LOG_PUMP;
use crate::profiler::network_types::{Hello, MatchResult, HELLO_PACKET};
use crate::profiler::state::{ChannelsIn, ProfilerState};
use crate::profiler::thread::{Command, command, FixedBufStr, run};
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
    channels: ChannelsIn
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
        //let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        //let listener = TcpListener::bind(addr)?;
        println!("Waiting for debugger to attach to {}...", port);
        //Block software until we receive a debugger connection.
        //let (mut client, _) = listener.accept()?;
        //handle_hello(&mut client)?;
        let logs = App::new(app_name).get_logs().ok().map(|v| v.to_owned());
        let (result_in, result_out) = oneshot::channel();
        //Got hit by https://github.com/rust-lang/rust/issues/100905
        let (state, channels) = ProfilerState::new(move |channels| {
            run(port, channels, logs, result_in);
            //let mut thread = Thread::new(client, channels, logs);
            //thread.run();
        });
        result_out.blocking_recv().unwrap()?;
        channels.control.blocking_send(command::Control::Project {
                app_name: FixedBufStr::from_str(app_name),
                name: FixedBufStr::from_str(crate_name),
                version: FixedBufStr::from_str(crate_version),
        }).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
        Ok(TracingSystem::with_destructor(
            Profiler { channels },
            Box::new(Guard(state)),
        ))
    }

    #[inline]
    fn span_command(&self, id: &Id, ty: command::SpanControl) {
        let _ = self.channels.span_control.blocking_send(command::Span { id: id.into_u64(), ty });
    }
}

impl Tracer for Profiler {
    fn enabled(&self) -> bool {
        true
    }

    fn span_create(&self, id: &Id, new: bool, parent: Option<Id>, span: &Attributes) {
        if new {
            self.span_command(id, command::SpanControl::Alloc {
                metadata: span.metadata()
            });
        }
        self.span_command(id, command::SpanControl::Init {
            parent: parent.map(|v| v.into_u64())
        });
        let mut visitor = ChannelVisitor::new(&self.channels.span_data, id.into_u64());
        span.record(&mut visitor);
    }

    fn span_values(&self, id: &Id, values: &Record) {
        let mut visitor = ChannelVisitor::new(&self.channels.span_data, id.into_u64());
        values.record(&mut visitor);
    }

    fn span_follows_from(&self, id: &Id, follows: &Id) {
        self.span_command(id, command::SpanControl::Follows {
            follows: follows.into_u64()
        });
    }

    fn event(&self, parent: Option<Id>, time: DateTime<Utc>, event: &Event) {
        let (target, module) = extract_target_module(event.metadata());
        let mut msg = LogMsg::new(target, tracing_level_to_log(event.metadata().level()));
        use std::fmt::Write;
        let _ = write!(msg, "{}: ", module.unwrap_or("main"));
        let mut visitor = FastVisitor::new(&mut msg, event.metadata().name());
        event.record(&mut visitor);
        let _ = self.channels.event.blocking_send(command::Event {
            id: parent.map(|v| (v.into_u64() >> 32) as u32).unwrap_or(0),
            message: msg,
            timestamp: time.timestamp()
        });
    }

    fn span_enter(&self, _: &Id) {
    }

    fn span_exit(&self, id: &Id, duration: Duration) {
        self.span_command(id, command::SpanControl::Exit {
            duration
        });
    }

    fn span_destroy(&self, id: &Id) {
        self.span_command(id, command::SpanControl::Free);
    }

    fn max_level_hint(&self) -> Option<Level> {
        None
    }
}
