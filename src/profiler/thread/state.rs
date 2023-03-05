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

use std::time::Duration;
use tokio::fs::File;
use tokio::io::BufWriter;

pub struct SpanData {
    pub full_run_count: u32,
    pub average_run_count: u32,
    pub has_overflowed: bool,
    pub min_time: Duration,
    pub max_time: Duration,
    pub total_time: Duration,
    pub max_rows: u32,
    pub max_average_points: u32,
    pub runs_file: Option<BufWriter<File>>,
    pub last_display_time: std::time::Instant
}

impl SpanData {
    pub fn new(runs_file: Option<BufWriter<File>>, max_rows: u32, max_average_points: u32) -> SpanData {
        SpanData {
            full_run_count: 0,
            average_run_count: 0,
            has_overflowed: false,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            total_time: Duration::ZERO,
            runs_file,
            last_display_time: std::time::Instant::now(),
            max_rows,
            max_average_points
        }
    }

    pub fn get_average(&self) -> Duration {
        if self.average_run_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.average_run_count
        }
    }

    pub fn update(&mut self, duration: &Duration) -> bool {
        //Avoid overflow of the full number of rows.
        if self.full_run_count < self.max_rows {
            self.full_run_count += 1;
        } else {
            self.has_overflowed = true;
        }
        //Avoid overflow of the average running time.
        self.average_run_count += 1;
        if self.average_run_count >= self.max_average_points {
            self.total_time = Duration::ZERO;
            self.average_run_count = 0;
        }
        if duration > &self.max_time {
            self.max_time = *duration;
        }
        if duration < &self.min_time {
            self.min_time = *duration;
        }
        self.total_time += *duration;
        let now = std::time::Instant::now();
        let duration = std::time::Instant::now() - self.last_display_time;
        if duration.as_millis() > 200 {
            self.last_display_time = now;
            true
        } else {
            false
        }
    }
}
