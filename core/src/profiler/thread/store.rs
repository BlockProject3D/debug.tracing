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

use super::{net::Net, state::SpanData};
use crate::profiler::thread::util::wrap_io_debug_error;
use crate::profiler::{log_msg::ProfilerRecord, network as net};
use bp3d_proto::message::payload::List;
use bp3d_proto::message::WriteSelf;
use std::{collections::HashMap, num::NonZeroU32};

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
        config: &net::client::Config<&[u8]>,
    ) -> SpanStore {
        let mut max_rows = config.get_record().get_max_rows();
        if max_rows > global_max_rows {
            max_rows = global_max_rows;
        }
        let mut period = config.get_period();
        if period < min_period {
            period = min_period;
        }
        SpanStore {
            span_data: HashMap::new(),
            max_rows,
            global_max_rows,
            max_average_points: config.get_max_average_points(),
            enable_recording: config.get_record().get_enable(),
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
            let msg = net::profiler::Dataset {
                section_id: k.get(),
                records: List::from_raw_parts(&v.runs_file, v.row_count as _),
            };
            wrap_io_debug_error!(
                net.network_write_dyn_payload(net::message::Type::ProfilerDataset, msg)
                    .await
            );
            v.row_count = 0;
            v.runs_file.clear();
        }
    }

    pub fn record(
        &mut self,
        log: ProfilerRecord,
    ) -> Option<net::profiler::SectionUpdate<[u8; net::profiler::SIZE_SECTION_UPDATE]>> {
        if let Some(data) = self.span_data.get_mut(&log.id()) {
            data.update(&log.get_duration(), self.max_average_points);
            if self.enable_recording && data.row_count < self.max_rows {
                data.row_count += 1;
                let buffer = &mut data.runs_file;
                let msg = net::profiler::Record {
                    header: log.header(),
                    fields: List::from_raw_parts(log.as_bytes(), log.var_count() as _),
                };
                let _ = msg.write_self(buffer);
            }
            let now = std::time::Instant::now();
            let duration = std::time::Instant::now() - data.last_display_time;
            if duration.as_millis() as u16 > self.period {
                data.last_display_time = now;
                let mut msg = net::profiler::SectionUpdate::new_on_stack();
                msg.set_id(log.id().get()).set_record_count(data.row_count);
                msg.get_average_time_mut().from_std(&data.get_average());
                msg.get_min_time_mut().from_std(&data.min_time);
                msg.get_max_time_mut().from_std(&data.max_time);
                Some(msg)
            } else {
                None
            }
        } else {
            None
        }
    }
}
