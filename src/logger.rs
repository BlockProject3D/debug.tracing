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

use std::borrow::Cow;
use std::fmt::Debug;
use std::time::Duration;
use bp3d_logger::Colors;
use dashmap::DashMap;
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::OffsetDateTimeExt;
use tracing_core::{Event, Field, Level};
use tracing_core::field::Visit;
use tracing_core::span::{Attributes, Id, Record};
use crate::core::{Tracer, TracingSystem};
use crate::util::{extract_target_module, Meta, tracing_level_to_log};

struct Visitor {
    msg: Option<String>,
    variables: Option<String>
}

impl Visitor {
    pub fn new() -> Visitor {
        Visitor {
            msg: None,
            variables: None
        }
    }

    pub fn get_variables(&self) -> Option<String> {
        if let Some(vars) = &self.variables {
            let mut vars = vars.clone();
            vars.truncate(vars.len() - 2);
            vars += " }";
            Some(vars)
        } else {
            None
        }
    }

    pub fn into_inner(self) -> (Option<String>, Option<String>) {
        if let Some(mut vars) = self.variables {
            vars.truncate(vars.len() - 2);
            vars += " }";
            (self.msg, Some(vars))
        } else {
            (self.msg, None)
        }
    }
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        let value = format!("{:?}", value);
        if field.name() == "message" {
            self.msg = Some(value);
        } else {
            let variables = self.variables.get_or_insert_with(|| String::from("{ "));
            *variables += &format!("{}: {:?}, ", field.name(), value);
        }
    }
}

struct SpanData {
    visitor: Visitor,
    metadata: Meta
}

impl SpanData {
    pub fn new(metadata: Meta) -> SpanData {
        SpanData {
            visitor: Visitor::new(),
            metadata
        }
    }
}

pub struct Logger {
    disabled: bool,
    level: Level,
    spans: DashMap<Id, SpanData>
}

impl Logger {
    pub fn new<T: bp3d_logger::GetLogs>(app: T) -> TracingSystem<Logger> {
        let disabled = bp3d_env::get_bool("LOG_DISABLE").unwrap_or(false);
        let level = bp3d_env::get("LOG").map(|v| v.to_lowercase())
            .map(Cow::Owned).unwrap_or("info".into());
        let level = match &*level {
            "error" => Level::ERROR,
            "warning" => Level::WARN,
            "info" => Level::INFO,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => Level::INFO
        };
        let always_stdout = bp3d_env::get_bool("LOG_STDOUT").unwrap_or(false);
        let colors = match bp3d_env::get_bool("LOG_COLOR") {
            None => bp3d_logger::Colors::Auto,
            Some(v) => match v {
                true => Colors::Enabled,
                false => Colors::Disabled
            }
        };
        let guard = bp3d_logger::Logger::new().smart_stderr(!always_stdout)
            .colors(colors).add_stdout().add_file(app).start();
        log::set_max_level(match level {
            Level::ERROR => log::LevelFilter::Error,
            Level::WARN => log::LevelFilter::Warn,
            Level::INFO => log::LevelFilter::Info,
            Level::DEBUG => log::LevelFilter::Debug,
            Level::TRACE => log::LevelFilter::Trace
        });
        TracingSystem::with_destructor(Logger {
            level,
            disabled,
            spans: DashMap::new()
        }, Box::new(guard))
    }
}

impl Tracer for Logger {
    fn enabled(&self) -> bool {
        !self.disabled
    }

    fn span_create(&self, id: &Id, _: bool, _: Option<Id>, attrs: &Attributes) {
        self.spans.insert(id.clone(), SpanData::new(attrs.metadata()));
    }

    fn span_values(&self, id: &Id, values: &Record) {
        let mut span_values = self.spans.get_mut(id).unwrap();
        values.record(&mut span_values.visitor);
    }

    fn span_follows_from(&self, _: &Id, _: &Id) {
    }

    fn event(&self, _: Option<Id>, time: OffsetDateTime, event: &Event) {
        let (target, module) = extract_target_module(event.metadata());
        let system_tz =
            time_tz::system::get_timezone().unwrap_or(time_tz::timezones::db::us::CENTRAL);
        let format = format_description!("[weekday repr:short] [month repr:short] [day] [hour repr:12]:[minute]:[second] [period case:upper]");
        //<error> is very unlikely to occur (only possibility is a weird io error).
        let formatted = time.to_timezone(system_tz).format(format)
            .unwrap_or_else(|_| "<error>".into());
        let mut visitor = Visitor::new();
        event.record(&mut visitor);
        let (msg, vars) = visitor.into_inner();
        let message = msg.map(Cow::Owned).unwrap_or(event.metadata().name().into());
        let msg = match vars {
            Some(v) => format!("({}) {}: {} {}", formatted, module.unwrap_or("main"), message, v),
            None => format!("({}) {}: {}", formatted, module.unwrap_or("main"), message)
        };
        let level = tracing_level_to_log(event.metadata().level());
        bp3d_logger::raw_log(bp3d_logger::LogMsg {
            msg,
            level,
            target: target.into()
        });
    }

    fn span_enter(&self, _: &Id) {
    }

    fn span_exit(&self, id: &Id, duration: Duration) {
        let data = self.spans.get(id).unwrap();
        let (target, module) = extract_target_module(data.metadata);
        let message = data.visitor.msg.as_deref().unwrap_or(data.metadata.name());
        let level = tracing_level_to_log(data.metadata.level());
        let msg = match data.visitor.get_variables() {
            Some(v) => format!("{}: The span '{} {}' finished in {}s", module.unwrap_or("main"), message, v, duration.as_secs_f64()),
            None => format!("{}: The span '{}' finished in {}s", module.unwrap_or("main"), message, duration.as_secs_f64()),
        };
        bp3d_logger::raw_log(bp3d_logger::LogMsg {
            msg,
            level,
            target: target.into()
        });
    }

    fn span_destroy(&self, id: Id) {
        self.spans.remove(&id);
    }

    fn max_level_hint(&self) -> Option<Level> {
        Some(self.level)
    }
}
