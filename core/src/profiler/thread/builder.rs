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

use std::thread::JoinHandle;
use tokio::sync::{mpsc, oneshot};
use crate::config::model::Config;
use crate::profiler::thread::command;
use crate::profiler::thread::core::{run, Levels};

pub struct ChannelsIn {
    pub execution: mpsc::Sender<command::Execution>,
    pub control: mpsc::Sender<command::Control>,
}

pub struct ChannelsOut {
    pub execution: mpsc::Receiver<command::Execution>,
    pub control: mpsc::Receiver<command::Control>,
}

pub struct RemoteDebuggerHandle {
    pub channels: ChannelsIn,
    pub levels: Levels,
    pub thread: JoinHandle<()>
}

pub struct Builder {
    pub port: u16,
    pub max_rows: u32,
    pub min_period: u16,
    pub buf_size: usize
}

impl Builder {
    pub fn new(config: &Config) -> Self {
        Self {
            port: config.get_profiler().get_port(),
            max_rows: config.get_profiler().get_max_rows(),
            min_period: config.get_profiler().get_min_period(),
            buf_size: config.get_profiler().get_buf_size()
        }
    }

    pub fn start(self) -> std::io::Result<RemoteDebuggerHandle> {
        println!("Waiting for debugger to attach to {}...", self.port);
        let (result_in, result_out) = oneshot::channel();
        let (ch_execution_in, ch_execution_out) = mpsc::channel(self.buf_size);
        let (ch_control_in, ch_control_out) = mpsc::channel(self.buf_size);
        let channels_in = ChannelsIn {
            execution: ch_execution_in,
            control: ch_control_in
        };
        let channels_out = ChannelsOut {
            execution: ch_execution_out,
            control: ch_control_out
        };
        let thread = std::thread::spawn(|| {
            run(self, channels_out, result_in);
        });
        let levels = result_out.blocking_recv().unwrap()?;
        Ok(RemoteDebuggerHandle {
            channels: channels_in,
            levels,
            thread,
        })
    }
}
