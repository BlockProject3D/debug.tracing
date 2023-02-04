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

use crate::util::{extract_target_module, tracing_level_to_log, Meta};
use bp3d_logger::LogMsg;
use std::fmt::Debug;
use std::fmt::Write;
use tracing_core::field::Visit;
use tracing_core::Field;

//#[derive(Clone)]
pub struct SpanVisitor {
    msg: LogMsg,
    message_written: bool,
    name: &'static str,
    module: &'static str,
}

impl SpanVisitor {
    pub fn new(metadata: Meta) -> SpanVisitor {
        let (target, module) = extract_target_module(metadata);
        let mut msg = LogMsg::new(target, tracing_level_to_log(metadata.level()));
        let module = module.unwrap_or("main");
        let _ = write!(msg, "{}: ", module);
        SpanVisitor {
            msg,
            message_written: false,
            name: metadata.name(),
            module,
        }
    }

    pub fn reset(&mut self) {
        self.msg.clear();
        self.message_written = false;
        let _ = write!(self.msg, "{}: ", self.module);
    }

    pub fn msg_mut(&mut self) -> &mut LogMsg {
        &mut self.msg
    }
}

impl Visit for SpanVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.msg, "{:?}", value);
            self.message_written = true;
        } else {
            if !self.message_written {
                let _ = write!(self.msg, "{}", self.name);
            }
            let _ = write!(self.msg, " {}={:?}", field.name(), value);
        }
    }
}

pub struct FastVisitor<'a> {
    msg: &'a mut LogMsg,
    message_written: bool,
    name: &'static str,
}

impl<'a> FastVisitor<'a> {
    pub fn new(msg: &'a mut LogMsg, name: &'static str) -> FastVisitor<'a> {
        FastVisitor {
            msg,
            message_written: false,
            name,
        }
    }
}

impl<'a> Visit for FastVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.msg, "{:?}", value);
            self.message_written = true;
        } else {
            if !self.message_written {
                let _ = write!(self.msg, "{}", self.name);
            }
            let _ = write!(self.msg, " {}={:?}", field.name(), value);
        }
    }
}
