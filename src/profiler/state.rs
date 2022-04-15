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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use crossbeam_channel::{bounded, Receiver, Sender};
use once_cell::sync::Lazy;
use crate::profiler::thread::Command;

const BUF_SIZE: usize = 128; // The maximum count of log messages in the channel.

pub struct ProfilerState {
    exited: AtomicBool,
    send_ch: Sender<Command>,
    recv_ch: Receiver<Command>,
    thread: Mutex<Option<JoinHandle<()>>>
}

impl ProfilerState {
    fn new() -> ProfilerState {
        let (send_ch, recv_ch) = bounded(BUF_SIZE);
        ProfilerState {
            exited: AtomicBool::new(false),
            send_ch,
            recv_ch,
            thread: Mutex::new(None)
        }
    }

    pub fn get() -> &'static ProfilerState {
        &PROFILER_STATE
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    pub fn get_channel(&self) -> (Sender<Command>, Receiver<Command>) {
        (self.send_ch.clone(), self.recv_ch.clone())
    }

    pub fn send(&self, cmd: Command) {
        // self.send_ch is a static (see PROFILER_STATE) so the channel cannot have been closed!
        unsafe { self.send_ch.send(cmd).unwrap_unchecked() }
    }

    pub fn assign_thread(&self, thread: JoinHandle<()>) {
        let mut lock = self.thread.lock().unwrap();
        if lock.is_some() {
            panic!("Cannot assign thread twice!");
        }
        *lock = Some(thread);
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

static PROFILER_STATE: Lazy<ProfilerState> = Lazy::new(ProfilerState::new);
