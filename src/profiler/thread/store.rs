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

use std::{collections::HashMap, io::Write, num::NonZeroU32};
use bytesutil::WriteExt;

use super::{net::Net, state::SpanData};
use crate::profiler::{log_msg::SpanLog, network_types as nt};
use crate::profiler::thread::util::wrap_io_debug_error;

pub struct SpanStore {
    span_data: HashMap<NonZeroU32, SpanData>,
    max_rows: u32,
    global_max_rows: u32,
    max_average_points: u32,
    enable_recording: bool,
    period: u16,
}

impl SpanStore {
    pub fn new(
        global_max_rows: u32,
        min_period: u16,
        config: &nt::message::ClientConfig,
    ) -> SpanStore {
        let mut max_rows = config.record.max_rows;
        if max_rows > global_max_rows {
            max_rows = global_max_rows;
        }
        let mut period = config.period;
        if period < min_period {
            period = min_period;
        }
        SpanStore {
            span_data: HashMap::new(),
            max_rows,
            global_max_rows,
            max_average_points: config.max_average_points,
            enable_recording: config.record.enable,
            period,
        }
    }

    pub fn reserve_span(&mut self, id: NonZeroU32) {
        self.span_data.insert(id, SpanData::new());
    }

    pub fn start_recording(&mut self, mut max_rows: u32) {
        if max_rows > self.global_max_rows {
            max_rows = self.global_max_rows;
        }
        self.max_rows = max_rows;
        self.enable_recording = true;
    }

    pub async fn stop_recording(&mut self, net: &mut Net<'_>) {
        self.enable_recording = false;
        for (k, v) in &mut self.span_data {
            let msg = nt::message::SpanDataset {
                id: k.get(),
                run_count: v.row_count
            };
            wrap_io_debug_error!(net.network_write_fixed_payload(msg, &v.runs_file).await);
            v.row_count = 0;
            v.runs_file.clear();
        }
    }

    pub fn record(&mut self, mut log: SpanLog) -> Option<nt::message::SpanUpdate> {
        if let Some(data) = self.span_data.get_mut(&log.id()) {
            data.update(&log.get_duration(), self.max_average_points);
            if self.enable_recording && data.row_count < self.max_rows {
                data.row_count += 1;
                let buffer = &mut data.runs_file;
                log.write_finish();
                let _ = buffer.write_le(log.as_bytes().len() as u16);
                let _ = buffer.write_all(log.as_bytes());
            }
            let now = std::time::Instant::now();
            let duration = std::time::Instant::now() - data.last_display_time;
            if duration.as_millis() as u16 > self.period {
                data.last_display_time = now;
                Some(nt::message::SpanUpdate {
                    id: log.id().get(),
                    run_count: data.row_count,
                    average_time: nt::message::Duration::from(&data.get_average()),
                    min_time: nt::message::Duration::from(&data.min_time),
                    max_time: nt::message::Duration::from(&data.max_time),
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}
