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

use crate::profiler::thread::Command;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use bp3d_logger::LogMsg;
use chrono::Utc;
use once_cell::sync::OnceCell;

const BUF_SIZE: usize = 256; // The maximum count of log messages in the channel.

static LOG_CHANNEL: OnceCell<Sender<Command>> = OnceCell::new();

pub fn send_message(message: LogMsg) {
    if let Some(val) = LOG_CHANNEL.get() {
        let _ = val.send(Command::Event {
            id: 0,
            message,
            timestamp: Utc::now().timestamp()
        });
    }
}

pub struct ProfilerState {
    exited: AtomicBool,
    send_ch: Sender<Command>,
    thread: Mutex<Option<JoinHandle<()>>>
}

impl ProfilerState {
    pub fn new<F: FnOnce(Receiver<Command>) + Send + 'static>(thread_fn: F) -> (ProfilerState, Sender<Command>) {
        let (send_ch, recv_ch) = bounded(BUF_SIZE);
        LOG_CHANNEL.set(send_ch.clone()).expect("Cannot initialize profiler more than once!");
        (ProfilerState {
            exited: AtomicBool::new(false),
            send_ch: send_ch.clone(),
            thread: Mutex::new(Some(std::thread::spawn(|| thread_fn(recv_ch)))),
        }, send_ch)
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    pub fn terminate(&self) {
        if self.is_exited() {
            return;
        }
        self.exited.store(true, Ordering::Relaxed);
        self.send_ch.send(Command::Terminate).unwrap();
        let thread = {
            let mut lock = self.thread.lock().unwrap();
            lock.take()
        };
        if let Some(thread) = thread {
            thread.join().unwrap();
        }
    }
}
