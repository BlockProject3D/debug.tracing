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

use crate::profiler::log_msg::{EventLog, SpanLog};
use std::fmt::Debug;
use std::num::NonZeroU32;
use tracing_core::field::Visit;
use tracing_core::Field;
use crate::profiler::network_types as nt;

pub struct SpanVisitor {
    msg: SpanLog,
    parent: Option<NonZeroU32>,
}

impl SpanVisitor {
    pub fn new(id: NonZeroU32, parent: Option<NonZeroU32>) -> SpanVisitor {
        SpanVisitor {
            msg: SpanLog::new(id),
            parent,
        }
    }

    pub fn reset(&mut self, parent: Option<NonZeroU32>) -> bool {
        self.msg.clear();
        if self.parent != parent {
            self.parent = parent;
            true
        } else {
            false
        }
    }

    pub fn msg_mut(&mut self) -> &mut SpanLog {
        &mut self.msg
    }
}

impl Visit for SpanVisitor {
    fn record_f64(&mut self, field: &Field, value: f64) {
        nt::log::Field::new(field.name(), value).write_into(&mut self.msg);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        nt::log::Field::new(field.name(), value).write_into(&mut self.msg);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        nt::log::Field::new(field.name(), value).write_into(&mut self.msg);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        nt::log::Field::new(field.name(), value).write_into(&mut self.msg);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        nt::log::Field::new(field.name(), value).write_into(&mut self.msg);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        nt::log::Field::new(field.name(), value).write_into(&mut self.msg);
    }
}

pub struct EventVisitor<'a> {
    msg: &'a mut EventLog,
}

impl<'a> EventVisitor<'a> {
    pub fn new(msg: &'a mut EventLog) -> EventVisitor {
        EventVisitor { msg }
    }
}

impl<'a> Visit for EventVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        nt::log::Field::new(field.name(), value).write_into(self.msg);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        nt::log::Field::new(field.name(), value).write_into(self.msg);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        nt::log::Field::new(field.name(), value).write_into(self.msg);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        nt::log::Field::new(field.name(), value).write_into(self.msg);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        nt::log::Field::new(field.name(), value).write_into(self.msg);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        nt::log::Field::new(field.name(), value).write_into(self.msg);
    }
}
