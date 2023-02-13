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

//! This module contains a log pump to be combined with Profiler in order to redirect the log
//! crate to the Profiler.

use bp3d_logger::LogMsg;
use crate::profiler::state::send_message;
use log::{Log, Metadata, Record};

pub struct LogPump;

pub static LOG_PUMP: LogPump = LogPump;

fn extract_target_module<'a>(record: &'a Record) -> (&'a str, Option<&'a str>) {
    let base_string = record.module_path().unwrap_or_else(|| record.target());
    let target = base_string
        .find("::")
        .map(|v| &base_string[..v])
        .unwrap_or(base_string);
    let module = base_string.find("::").map(|v| &base_string[(v + 2)..]);
    (target, module)
}

impl Log for LogPump {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let (target, module) = extract_target_module(record);
        let mut msg = LogMsg::new(target, record.level());
        use std::fmt::Write;
        let _ = write!(msg, "{}: {}", module.unwrap_or("main"), record.args());
        send_message(msg);
    }

    fn flush(&self) {}
}
