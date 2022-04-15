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

use log::{Log, Metadata, Record};
use time::OffsetDateTime;
use tracing_core::dispatcher::get_default;
use crate::profiler::state::ProfilerState;
use crate::profiler::thread::{Command, Event};

pub struct LogPump;

pub static LOG_PUMP: LogPump = LogPump;

impl Log for LogPump {
    fn enabled(&self, _: &Metadata) -> bool {
        !ProfilerState::get().is_exited()
    }

    fn log(&self, record: &Record) {
        if ProfilerState::get().is_exited() {
            return;
        }
        let current = get_default(|v| v.current_span());
        let metadata = crate::profiler::network_types::Metadata::from_log(record);
        let time = OffsetDateTime::now_utc().unix_timestamp();
        let message = format!("{}", record.args());
        ProfilerState::get().send(Command::Event(Event::Owned {
            span: current.id().map(|v| v.into_u64()),
            metadata,
            time,
            value_set: Vec::new(),
            message: Some(message)
        }));
    }

    fn flush(&self) {}
}
