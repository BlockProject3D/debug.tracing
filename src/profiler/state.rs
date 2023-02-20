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

use tokio::sync::mpsc;
use crate::profiler::thread::command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use once_cell::sync::OnceCell;
use crate::profiler::log_msg::{EventLog, SpanLog};

const BUF_SIZE: usize = 256; // The maximum count of log messages in the channel.

static LOG_CHANNEL: OnceCell<mpsc::Sender<EventLog>> = OnceCell::new();

pub fn send_message(message: &EventLog) {
    if let Some(val) = LOG_CHANNEL.get() {
        let _ = val.try_send(message.clone());
    }
}

pub struct ChannelsIn {
    pub span: mpsc::Sender<SpanLog>,
    pub span_control: mpsc::Sender<command::Span<command::SpanControl>>,
    pub event: mpsc::Sender<EventLog>,
    pub control: mpsc::Sender<command::Control>,
}

pub struct ChannelsOut {
    pub span: mpsc::Receiver<SpanLog>,
    pub span_control: mpsc::Receiver<command::Span<command::SpanControl>>,
    pub event: mpsc::Receiver<EventLog>,
    pub control: mpsc::Receiver<command::Control>
}

pub struct ProfilerState {
    exited: AtomicBool,
    send_ch: mpsc::Sender<command::Control>,
    thread: Mutex<Option<JoinHandle<()>>>
}

impl ProfilerState {
    pub fn new<F: FnOnce(ChannelsOut) + Send + 'static>(thread_fn: F) -> (ProfilerState, ChannelsIn) {
        let (ch_span_control_in, ch_span_control_out) = mpsc::channel(BUF_SIZE);
        let (ch_span_in, ch_span_out) = mpsc::channel(BUF_SIZE);
        let (ch_event_in, ch_event_out) = mpsc::channel(BUF_SIZE);
        let (ch_control_in, ch_control_out) = mpsc::channel(BUF_SIZE);
        LOG_CHANNEL.set(ch_event_in.clone()).expect("Cannot initialize profiler more than once!");
        (ProfilerState {
            exited: AtomicBool::new(false),
            send_ch: ch_control_in.clone(),
            thread: Mutex::new(Some(std::thread::spawn(|| thread_fn(ChannelsOut {
                span: ch_span_out,
                span_control: ch_span_control_out,
                event: ch_event_out,
                control: ch_control_out
            })))),
        }, ChannelsIn {
            span: ch_span_in,
            span_control: ch_span_control_in,
            event: ch_event_in,
            control: ch_control_in
        })
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    pub fn terminate(&self) {
        if self.is_exited() {
            return;
        }
        self.exited.store(true, Ordering::Relaxed);
        self.send_ch.blocking_send(command::Control::Terminate).unwrap();
        let thread = {
            let mut lock = self.thread.lock().unwrap();
            lock.take()
        };
        if let Some(thread) = thread {
            thread.join().unwrap();
        }
    }
}
