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
use crate::util::{extract_target_module, tracing_level_to_log};
use crate::visitor::{FastVisitor, SpanVisitor};
use bp3d_logger::{Colors, LogMsg};
use chrono::{DateTime, Local, Utc};
use dashmap::DashMap;
use std::borrow::Cow;
use std::fmt::Write;
use std::time::Duration;
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Level};

pub struct Logger {
    disabled: bool,
    level: Level,
    spans: DashMap<Id, SpanVisitor>,
}

impl Logger {
    pub fn new<T: bp3d_logger::GetLogs>(app: T) -> TracingSystem<Logger> {
        let disabled = bp3d_env::get_bool("LOG_DISABLE").unwrap_or(false);
        let level = bp3d_env::get("LOG")
            .map(|v| v.to_lowercase())
            .map(Cow::Owned)
            .unwrap_or("info".into());
        let level = match &*level {
            "error" => Level::ERROR,
            "warning" => Level::WARN,
            "info" => Level::INFO,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => Level::INFO,
        };
        let always_stdout = bp3d_env::get_bool("LOG_STDOUT").unwrap_or(false);
        let colors = match bp3d_env::get_bool("LOG_COLOR") {
            None => Colors::Auto,
            Some(v) => match v {
                true => Colors::Enabled,
                false => Colors::Disabled,
            },
        };
        let guard = bp3d_logger::Logger::new()
            .smart_stderr(!always_stdout)
            .colors(colors)
            .add_stdout()
            .add_file(app)
            .start();
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
                disabled,
                spans: DashMap::new(),
            },
            Box::new(guard),
        )
    }
}

impl Tracer for Logger {
    fn enabled(&self) -> bool {
        !self.disabled
    }

    fn span_create(&self, id: &Id, _: bool, _: Option<Id>, attrs: &Attributes) {
        if let Some(mut data) = self.spans.get_mut(id) {
            data.reset();
            attrs.record(&mut *data);
        } else {
            let mut data = SpanVisitor::new(attrs.metadata());
            attrs.record(&mut data);
            self.spans.insert(id.clone(), data);
        }
    }

    fn span_values(&self, id: &Id, values: &Record) {
        let mut span_values = self.spans.get_mut(id).unwrap();
        values.record(&mut *span_values);
    }

    fn span_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, _: Option<Id>, time: DateTime<Utc>, event: &Event) {
        let (target, module) = extract_target_module(event.metadata());
        let time = DateTime::<Local>::from(time);
        let formatted = time.format("%a %b %d %Y %I:%M:%S %P");
        let mut msg = LogMsg::new(target, tracing_level_to_log(event.metadata().level()));
        let _ = write!(msg, "({}) {}: ", formatted, module.unwrap_or("main"));
        let mut visitor = FastVisitor::new(&mut msg, event.metadata().name());
        event.record(&mut visitor);
        bp3d_logger::raw_log(&msg);
    }

    fn span_enter(&self, _: &Id) {}

    fn span_exit(&self, id: &Id, duration: Duration) {
        let mut data = self.spans.get_mut(id).unwrap();
        let msg = data.msg_mut();
        let _ = write!(msg, ": span finished in {:.2}s", duration.as_secs_f64());
        bp3d_logger::raw_log(&msg);
    }

    fn span_destroy(&self, _: &Id) {
        //self.spans.remove(&id);
    }

    fn max_level_hint(&self) -> Option<Level> {
        Some(self.level)
    }
}
