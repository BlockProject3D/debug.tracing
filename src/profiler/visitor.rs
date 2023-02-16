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
use tokio::sync::mpsc;
use tracing_core::field::Visit;
use tracing_core::Field;
use crate::profiler::thread::{Command, FixedBufStr, FixedBufValue, command};
use crate::util::SpanId;

pub struct ChannelVisitor<'a> {
    sender: &'a mpsc::Sender<command::Span<command::SpanControl>>,
    span: SpanId
}

impl<'a> ChannelVisitor<'a> {
    pub fn new(sender: &'a mpsc::Sender<command::Span<command::SpanControl>>, span: SpanId) -> Self {
        Self {
            sender,
            span
        }
    }

    #[inline]
    fn value(&self, key: &'static str, value: FixedBufValue)
    {
        let _ = self.sender.blocking_send(command::Span {
            id: self.span,
            ty: command::SpanControl::Value {
                key,
                value
            }
        });
    }
}

impl<'a> Visit for ChannelVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.value(field.name(), FixedBufValue::Float(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.value(field.name(), FixedBufValue::Signed(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.value(field.name(), FixedBufValue::Unsigned(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.value(field.name(), FixedBufValue::Bool(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            let _ = self.sender.blocking_send(command::Span {
                id: self.span,
                ty: command::SpanControl::Message {
                    message: FixedBufStr::from_str(value)
                }
            });
        } else {
            self.value(field.name(), FixedBufValue::String(FixedBufStr::from_str(value)));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = self.sender.blocking_send(command::Span {
                id: self.span,
                ty: command::SpanControl::Message {
                    message: FixedBufStr::from_debug(value)
                }
            });
        } else {
            self.value(field.name(), FixedBufValue::String(FixedBufStr::from_debug(value)));
        }
    }
}
