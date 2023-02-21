// Copyright (c) 2023, BlockProject 3D
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
use crate::profiler::state::{ChannelsIn, ProfilerState};
use crate::profiler::thread::{command, FixedBufStr, run};
use crate::profiler::visitor::{EventVisitor, SpanVisitor};
use crate::profiler::DEFAULT_PORT;
use chrono::{DateTime, Utc};
use std::time::Duration;
use bp3d_fs::dirs::App;
use dashmap::DashMap;
use tracing_core::span::{Attributes, Record};
use tracing_core::{Event, Level};
use crate::profiler::log_msg::{EventLog};
use crate::util::{extract_target_module, SpanId};
use crate::profiler::network_types as nt;

struct Guard(ProfilerState);

impl Drop for Guard {
    fn drop(&mut self) {
        self.0.terminate()

    }
}

pub struct Profiler {
    spans: DashMap<SpanId, SpanVisitor>,
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
        println!("Waiting for debugger to attach to {}...", port);
        //Block software until we receive a debugger connection.
        let logs = App::new(app_name).get_logs().ok().map(|v| v.to_owned());
        let (result_in, result_out) = oneshot::channel();
        //Got hit by https://github.com/rust-lang/rust/issues/100905
        let (state, channels) = ProfilerState::new(move |channels| {
            run(port, channels, logs, result_in);
        });
        result_out.blocking_recv().unwrap()?;
        channels.control.blocking_send(command::Control::Project {
                app_name: FixedBufStr::from_str(app_name),
                name: FixedBufStr::from_str(crate_name),
                version: FixedBufStr::from_str(crate_version),
        }).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
        Ok(TracingSystem::with_destructor(
            Profiler {
                spans: DashMap::new(),
                channels
            },
            Box::new(Guard(state)),
        ))
    }

    #[inline]
    fn span_command(&self, cmd: command::Span) {
        let _ = self.channels.span.blocking_send(cmd);
    }
}

impl Tracer for Profiler {
    fn enabled(&self) -> bool {
        true
    }

    fn span_create(&self, id: &SpanId, new: bool, parent: Option<SpanId>, attrs: &Attributes) {
        let node_id = id.get_id();
        if new {
            self.span_command(command::Span::Alloc { id: node_id, metadata: attrs.metadata() })
        }
        let parent = parent.map(|v| v.get_id());
        if let Some(mut data) = self.spans.get_mut(id) {
            if data.reset(parent) {
                self.span_command(command::Span::UpdateParent {
                    id: node_id,
                    parent
                });
            }
            attrs.record(&mut *data);
        } else {
            let mut data = SpanVisitor::new(id.get_id(), parent);
            attrs.record(&mut data);
            self.spans.insert(*id, data);
            self.span_command(command::Span::UpdateParent {
                id: node_id,
                parent
            });
        }
    }

    fn span_values(&self, id: &SpanId, values: &Record) {
        let mut span_values = self.spans.get_mut(id).unwrap();
        values.record(&mut *span_values);
    }

    fn span_follows_from(&self, id: &SpanId, follows: &SpanId) {
        self.span_command(command::Span::Follows {
            id: *id, follows: *follows
        });
    }

    fn event(&self, parent: Option<SpanId>, time: DateTime<Utc>, event: &Event) {
        let (target, module) = extract_target_module(event.metadata());
        let mut msg = EventLog::new(parent.map(|v| v.get_id()),
                                    time.timestamp(),
                                    nt::header::Level::from_tracing(*event.metadata().level()));
        use std::fmt::Write;
        let mut visitor = EventVisitor::new(&mut msg);
        event.record(&mut visitor);
        let _ = write!(msg, ",{},{}", module.unwrap_or("main"), target);
        self.span_command(command::Span::Event(msg));
    }

    fn span_enter(&self, _: &SpanId) {
    }

    fn span_exit(&self, id: &SpanId, duration: Duration) {
        let mut span = self.spans.get_mut(id).unwrap();
        let msg = span.msg_mut();
        msg.set_duration(&duration);
        self.span_command(command::Span::Log(msg.clone()));
    }

    fn span_destroy(&self, _: &SpanId) {
    }

    fn max_level_hint(&self) -> Option<Level> {
        None
    }
}
