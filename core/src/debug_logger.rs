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

use std::collections::HashMap;
use std::fmt::Arguments;
use std::num::NonZeroU32;
use bp3d_debug::field::Field;
use bp3d_debug::logger::{Callsite, Level, Logger};
use bp3d_debug::profiler::Profiler;
use bp3d_debug::profiler::section::Section;
use bp3d_debug::trace::Tracer;
use bp3d_logger::LogMsg;
use std::fmt::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;
use time::OffsetDateTime;
use crate::tracer_base::BaseTracer;

struct Profiler1 {
    map: RwLock<HashMap<NonZeroU32, &'static Section>>,
    cur_section: AtomicU32
}

pub struct Debugger {
    log: bp3d_logger::Logger,
    profiler: Profiler1,
    tracer: BaseTracer<LogMsg>
}

impl Logger for Debugger {
    fn log(&self, callsite: &'static Callsite, msg: Arguments, fields: &[Field]) {
        let mut lmsg = LogMsg::new(*callsite.location(), callsite.level());
        let _ = write!(lmsg, "{}", msg);
        for field in fields {
            let _ = write!(lmsg, ", {} = {}", field.name(), field.value());
        }
        self.log.log(&lmsg);
    }
}

impl Profiler for Debugger {
    fn section_register(&self, section: &'static Section) -> NonZeroU32 {
        let id = unsafe { NonZeroU32::new_unchecked(self.profiler.cur_section.fetch_add(1, Ordering::Relaxed)) };
        let mut guard = self.profiler.map.write().unwrap();
        guard.insert(id, section);
        id
    }

    fn section_record(&self, id: NonZeroU32, start: u64, end: u64, fields: &[Field]) {
        let guard = self.profiler.map.read().unwrap();
        let section = guard[&id];
        let level = match section.level() {
            bp3d_debug::profiler::section::Level::Critical => Level::Trace,
            bp3d_debug::profiler::section::Level::Periodic => Level::Debug,
            bp3d_debug::profiler::section::Level::Event => Level::Info
        };
        let mut msg = LogMsg::new(*section.location(), level);
        let _ = write!(msg, "[Profiler] Section {} took {}µs", section.name(), (end - start) / 1000);
        for field in fields {
            let _ = write!(msg, ", {} = {}", field.name(), field.value());
        }
        self.log.log(&msg);
    }
}

impl Tracer for Debugger {
    fn register_callsite(&self, callsite: &'static bp3d_debug::trace::span::Callsite) -> NonZeroU32 {
        self.tracer.register_callsite(callsite)
    }

    fn span_create(&self, callsite: NonZeroU32, fields: &[Field]) -> NonZeroU32 {
        let callsite1 = self.tracer.get_callsite(callsite);
        let mut msg = LogMsg::new(*callsite1.location(), Level::Debug);
        let _ = write!(msg, "[Tracer] Span {}", callsite1.name());
        for field in fields {
            let _ = write!(msg, ", {} = {}", field.name(), field.value());
        }
        let (id, _) = self.tracer.create_span(callsite, msg);
        id
    }

    fn span_enter(&self, id: NonZeroU32) {
        let mut data = self.tracer.span_enter(id);
        data.set_time(OffsetDateTime::now_utc()); // Set the current time for the LogMsg.
    }

    fn span_record(&self, id: NonZeroU32, fields: &[Field]) {
        let mut data = self.tracer.get_data(id);
        for field in fields {
            let _ = write!(data, ", {} = {}", field.name(), field.value());
        }
    }

    fn span_exit(&self, id: NonZeroU32) {
        let mut data = self.tracer.span_exit(id);
        let motherfuckingrust = data.end();
        let motherfuckingrust2 = data.start();
        let motherfuckingrust3 = data.order();
        let _ = write!(data, " (instance #{}) took {}ms", motherfuckingrust3, (motherfuckingrust - motherfuckingrust2) / 1000000);
        data.clear();
    }
}
