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

use std::io::{Cursor, Write};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use crate::profiler::thread::command::FixedBufValue;
use crate::profiler::thread::util::FixedBufStr;

pub struct SpanData {
    pub run_count: u32,
    pub instance_count: u32,
    pub has_overflowed: bool,
    pub min_time: Duration,
    pub max_time: Duration,
    pub total_time: Duration,
    pub parent: Option<u32>,
    pub name: &'static str,
    pub runs_file: Option<BufWriter<File>>
}

impl SpanData {
    pub fn new(name: &'static str, runs_file: Option<BufWriter<File>>) -> SpanData {
        SpanData {
            run_count: 0,
            instance_count: 0,
            has_overflowed: false,
            min_time: Duration::ZERO,
            max_time: Duration::MAX,
            total_time: Duration::ZERO,
            parent: None,
            name,
            runs_file
        }
    }

    pub fn get_average(&self) -> Duration {
        if self.run_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.run_count
        }
    }
}

pub struct SpanInstance {
    message_written: bool,
    pub csv_row: Cursor<[u8; 1024]>,
    variables: Cursor<[u8; 1024]>
}

impl SpanInstance {
    pub fn new() -> SpanInstance {
        SpanInstance {
            message_written: false,
            csv_row: Cursor::new([0; 1024]),
            variables: Cursor::new([0; 1024])
        }
    }

    pub async fn write<T: Unpin + AsyncWriteExt>(&self, mut file: T) {
        let row_start = &self.csv_row.get_ref()[..self.csv_row.position() as _];
        let row_end = &self.variables.get_ref()[..self.variables.position() as _];
        let _ = file.write(row_start).await;
        let _ = file.write(row_end).await;
        let _ = file.write(b"\n").await;
    }

    pub fn reset(&mut self) {
        self.csv_row.set_position(0);
        self.variables.set_position(0);
        self.message_written = false;
    }

    pub fn finish(&mut self, duration: &Duration, name: &str) {
        if !self.message_written {
            let _ = write!(self.csv_row, "{}", name);
        }
        let _ = write!(self.csv_row, ",{},{},{}",
                       duration.as_secs(), duration.subsec_millis(),
                       duration.subsec_micros() - (duration.subsec_millis() * 1000));
    }

    pub fn append_value(&mut self, name: &'static str, value: &FixedBufValue) {
        let _ = match value {
            FixedBufValue::Float(v) => write!(self.variables, ",\"{}\"={}", name, v),
            FixedBufValue::Signed(v) => write!(self.variables, ",\"{}\"={}", name, v),
            FixedBufValue::Unsigned(v) => write!(self.variables, ",\"{}\"={}", name, v),
            FixedBufValue::String(v) => write!(self.variables, ",\"{}\"=\"{}\"", name, v.str()),
            FixedBufValue::Bool(v) => write!(self.variables, ",\"{}\"={}", name, v)
        };
    }

    pub fn append_message(&mut self, message: &FixedBufStr<63>) {
        let _ = write!(self.csv_row, "\"{}\"", message.str());
        self.message_written = true;
    }
}
