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

use std::fmt::Debug;
use crossbeam_channel::Sender;
use tracing_core::field::Visit;
use tracing_core::Field;
use crate::profiler::thread::{Command, FixedBufStr, FixedBufValue};

pub struct ChannelVisitor<'a> {
    sender: &'a Sender<Command>,
    span: u64
}

impl<'a> ChannelVisitor<'a> {
    pub fn new(sender: &'a Sender<Command>, span: u64) -> Self {
        Self {
            sender,
            span
        }
    }
}

impl<'a> Visit for ChannelVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        let _ = self.sender.send(Command::SpanValue {
            span: self.span,
            key: field.name(),
            value: FixedBufValue::Float(value)
        });
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        let _ = self.sender.send(Command::SpanValue {
            span: self.span,
            key: field.name(),
            value: FixedBufValue::Signed(value)
        });
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        let _ = self.sender.send(Command::SpanValue {
            span: self.span,
            key: field.name(),
            value: FixedBufValue::Unsigned(value)
        });
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        let _ = self.sender.send(Command::SpanValue {
            span: self.span,
            key: field.name(),
            value: FixedBufValue::Bool(value)
        });
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            let _ = self.sender.send(Command::SpanMessage {
                span: self.span,
                message: FixedBufStr::from_str(value)
            });
        } else {
            let _ = self.sender.send(Command::SpanValue {
                span: self.span,
                key: field.name(),
                value: FixedBufValue::String(FixedBufStr::from_str(value))
            });
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = self.sender.send(Command::SpanMessage {
                span: self.span,
                message: FixedBufStr::from_debug(value)
            });
        } else {
            let _ = self.sender.send(Command::SpanValue {
                span: self.span,
                key: field.name(),
                value: FixedBufValue::String(FixedBufStr::from_debug(value))
            });
        }
    }
}
