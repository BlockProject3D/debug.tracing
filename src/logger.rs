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

use crate::config::model::Config;
use crate::core::{Tracer, TracingSystem};
use crate::util::{extract_target_module, SpanId, tracing_level_to_log};
use crate::visitor::{FastVisitor, SpanVisitor};
use bp3d_logger::{Colors, LogMsg};
use chrono::{DateTime, Local, Utc};
use dashmap::DashMap;
use std::fmt::Write;
use std::time::Duration;
use tracing_core::span::{Attributes, Record};
use tracing_core::{Event, Level};

pub struct Logger {
    level: Level,
    spans: DashMap<SpanId, SpanVisitor>,
    time_format: String
}

impl Logger {
    pub fn new<T: bp3d_logger::GetLogs>(app: T, config: &Config) -> TracingSystem<Logger> {
        let level = config.get_logger().get_level().to_tracing();
        let mut guard = bp3d_logger::Logger::new();
        if let Some(console) = &config.get_logger().console {
            let colors = match console.get_color() {
                crate::config::model::Color::Auto => Colors::Auto,
                crate::config::model::Color::Always => Colors::Enabled,
                crate::config::model::Color::Never => Colors::Disabled
            };    
            guard = guard
                .smart_stderr(console.get_stderr())
                .colors(colors)
                .add_stdout()
        }
        if config.get_logger().file.is_some() {
            guard = guard.add_file(app)
        }
        let guard = guard.start();
        log::set_max_level(match level {
            Level::ERROR => log::LevelFilter::Error,
            Level::WARN => log::LevelFilter::Warn,
            Level::INFO => log::LevelFilter::Info,
            Level::DEBUG => log::LevelFilter::Debug,
            Level::TRACE => log::LevelFilter::Trace,
        });
        TracingSystem::with_destructor(
            Logger {
                level,
                spans: DashMap::new(),
                time_format: config.get_logger().get_time_format().into()
            },
            Box::new(guard),
        )
    }
}

impl Tracer for Logger {
    fn enabled(&self) -> bool {
        true
    }

    fn span_create(&self, id: &SpanId, _: bool, _: Option<SpanId>, attrs: &Attributes) {
        if let Some(mut data) = self.spans.get_mut(id) {
            data.reset();
            attrs.record(&mut *data);
        } else {
            let mut data = SpanVisitor::new(attrs.metadata());
            attrs.record(&mut data);
            self.spans.insert(*id, data);
        }
    }

    fn span_values(&self, id: &SpanId, values: &Record) {
        let mut span_values = self.spans.get_mut(id).unwrap();
        values.record(&mut *span_values);
    }

    fn span_follows_from(&self, _: &SpanId, _: &SpanId) {}

    fn event(&self, _: Option<SpanId>, time: DateTime<Utc>, event: &Event) {
        let (target, module) = extract_target_module(event.metadata());
        let time = DateTime::<Local>::from(time);
        let formatted = time.format(&self.time_format);
        let mut msg = LogMsg::new(target, tracing_level_to_log(event.metadata().level()));
        let _ = write!(msg, "({}) {}: ", formatted, module.unwrap_or("main"));
        let mut visitor = FastVisitor::new(&mut msg, event.metadata().name());
        event.record(&mut visitor);
        bp3d_logger::raw_log(&msg);
    }

    fn span_enter(&self, _: &SpanId) {}

    fn span_exit(&self, id: &SpanId, duration: Duration) {
        let mut data = self.spans.get_mut(id).unwrap();
        let msg = data.msg_mut();
        let _ = write!(msg, ": span finished in {:.2}s", duration.as_secs_f64());
        bp3d_logger::raw_log(&msg);
    }

    fn span_destroy(&self, _: &SpanId) {
        //self.spans.remove(&id);
    }

    fn max_level_hint(&self) -> Option<Level> {
        Some(self.level)
    }
}
