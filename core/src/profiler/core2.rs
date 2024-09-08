// Copyright (c) 2024, BlockProject 3D
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

use std::cell::RefCell;
use std::fmt::Arguments;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use bp3d_debug::field::Field;
use bp3d_debug::logger::{Callsite, Logger};
use bp3d_debug::profiler::Profiler;
use bp3d_debug::profiler::section::Section;
use bp3d_debug::trace::span::Id;
use bp3d_debug::trace::Tracer;
use bp3d_os::time::LocalOffsetDateTime;
use bp3d_util::format::FixedBufStr;
use time::OffsetDateTime;
use crate::config::model::Config;
use crate::profiler::log_msg::{EventLog, FieldsetRecord, ProfilerRecord};
use crate::profiler::network as net;
use crate::profiler::thread::{Builder, ChannelsIn, Levels};
use crate::profiler::thread::command::{Control, Execution};
use crate::profiler::util::write_fields;
use crate::tracer_base::BaseTracer;
use crate::profiler::util::WriteField;

thread_local! {
    static SPAN_STACK: RefCell<Vec<Id>> = RefCell::new(Vec::new());
}

pub static REMOTE_DEBUGGER: OnceLock<RemoteDebugger> = OnceLock::new();

//TODO: Implement termination

pub struct RemoteDebugger {
    channels: ChannelsIn,
    cur_section: AtomicU32,
    tracer: BaseTracer<FieldsetRecord>,
    profiler_level: i8,
    event_level: u8
}

impl RemoteDebugger {
    pub fn new(
        app_name: &str,
        crate_name: &str,
        crate_version: &str,
        config: &Config,
    ) -> std::io::Result<RemoteDebugger> {
        let handle = Builder::new(config).start()?;
        handle.channels.control
            .blocking_send(Control::Project {
                app_name: FixedBufStr::from_str(app_name),
                name: FixedBufStr::from_str(crate_name),
                version: FixedBufStr::from_str(crate_version),
            })
            .unwrap();
        let profiler_level = match handle.levels.section {
            net::profiler::Level::None => bp3d_debug::profiler::section::Level::Critical as i8,
            net::profiler::Level::Disabled => -1,
            net::profiler::Level::Critical => bp3d_debug::profiler::section::Level::Critical as i8,
            net::profiler::Level::Periodic => bp3d_debug::profiler::section::Level::Periodic as i8,
            net::profiler::Level::Event => bp3d_debug::profiler::section::Level::Event as i8
        };
        let event_level = match handle.levels.event {
            net::event::Level::None => bp3d_debug::logger::Level::Trace as u8,
            net::event::Level::Disabled => 0,
            net::event::Level::Trace => bp3d_debug::logger::Level::Trace as u8,
            net::event::Level::Debug => bp3d_debug::logger::Level::Debug as u8,
            net::event::Level::Info => bp3d_debug::logger::Level::Info as u8,
            net::event::Level::Warn => bp3d_debug::logger::Level::Warn as u8,
            net::event::Level::Error => bp3d_debug::logger::Level::Error as u8
        };
        Ok(RemoteDebugger {
            channels: handle.channels,
            cur_section: AtomicU32::new(1),
            tracer: BaseTracer::new(),
            profiler_level,
            event_level
        })
    }
}

impl Logger for RemoteDebugger {
    fn log(&self, callsite: &'static Callsite, msg: Arguments, fields: &[Field]) {
        let level = callsite.level() as u8;
        if level < self.event_level {
            return;
        }
        let span = SPAN_STACK.with(|v| v.borrow().last().map(|v| *v));
        let timestamp = OffsetDateTime::now_local().unwrap_or_else(|| OffsetDateTime::now_utc()).unix_timestamp_nanos() / 1000;
        let mut log = EventLog::new(span, timestamp as _, callsite.level(), *callsite.location());
        let mut buffer = [0; 1];
        // Amazingly broken Rust is far too stupid to figure out that write_field is being called on &self!!!
        (&msg).write_field("message", &mut buffer, &mut log);
        write_fields(fields, &mut log);
        let _ = self.channels.execution.send(Execution::Event(log));
    }
}

impl Profiler for RemoteDebugger {
    fn section_register(&self, section: &'static Section) -> NonZeroU32 {
        let level = section.level() as i8;
        if level < self.profiler_level {
            return unsafe { NonZeroU32::new_unchecked(u32::MAX) }
        }
        let id = unsafe { NonZeroU32::new_unchecked(self.cur_section.fetch_add(1, Ordering::Relaxed)) };
        let _ = self.channels.control.send(Control::RegisterSection {
            section,
            id,
            parent: section.parent().map(|v| *v.get_id())
        });
        id
    }

    fn section_record(&self, id: NonZeroU32, start: u64, end: u64, fields: &[Field]) {
        if id.get() == u32::MAX {
            return;
        }
        let mut record = ProfilerRecord::new(id, start, end);
        write_fields(fields, &mut record);
        record.add_vars(fields.len() as _);
        let _ = self.channels.execution.send(Execution::ProfilerRecord(record));
    }
}

impl Tracer for RemoteDebugger {
    fn register_callsite(&self, callsite: &'static bp3d_debug::trace::span::Callsite) -> NonZeroU32 {
        let id = self.tracer.register_callsite(callsite);
        let _ = self.channels.control.send(Control::RegisterSpan {
            callsite,
            id
        });
        id
    }

    fn span_create(&self, callsite: NonZeroU32, fields: &[Field]) -> NonZeroU32 {
        let (instance, mut fieldset) = self.tracer.create_span(callsite, FieldsetRecord::new());
        let id = Id::new(callsite, instance);
        fieldset.set_id(id);
        write_fields(fields, &mut **fieldset);
        fieldset.add_vars(fields.len() as _);
        instance
    }

    fn span_enter(&self, id: Id) {
        SPAN_STACK.with(|v| v.borrow_mut().push(id));
        let fieldset = self.tracer.span_enter(id);
        let _ = self.channels.execution.send(Execution::SpanEnter {
            fields: fieldset.clone(),
            start: fieldset.start()
        });
    }

    fn span_record(&self, id: Id, fields: &[Field]) {
        let mut data = self.tracer.get_data(id);
        if data.uses() > 1 {
            let mut fieldset = FieldsetRecord::new();
            fieldset.set_id(id);
            write_fields(fields, &mut fieldset);
            fieldset.add_vars(fields.len() as _);
            let _ = self.channels.execution.send(Execution::SpanRecord(fieldset));
        } else {
            write_fields(fields, &mut **data);
            data.add_vars(fields.len() as _);
        }
    }

    fn span_exit(&self, id: Id) {
        SPAN_STACK.with(|v| v.borrow_mut().pop());
        let data = self.tracer.span_exit(id);
        let _ = self.channels.execution.send(Execution::SpanExit {
            id,
            end: data.end()
        });
    }

    fn span_destroy(&self, id: Id) {
        self.tracer.destroy_span(id);
    }
}
