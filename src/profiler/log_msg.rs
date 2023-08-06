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

use crate::profiler::network_types as nt;
use std::fmt::Result;
use std::fmt::Write;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::time::Duration;

pub const CTRL_LOG_SPAN: usize = std::mem::size_of::<NonZeroU32>() + std::mem::size_of::<u16>();
pub const CTRL_LOG_EVENT: usize = std::mem::size_of::<i64>()
    + std::mem::size_of::<Option<NonZeroU32>>()
    + std::mem::size_of::<u16>()
    + 1;

macro_rules! impl_log_msg {
    ($name: ident) => {
        impl $name {
            pub fn msg(&self) -> &str {
                unsafe {
                    std::str::from_utf8_unchecked(std::mem::transmute(
                        &self.buffer[..self.msg_len as usize],
                    ))
                }
            }

            pub unsafe fn write(&mut self, buf: &[u8]) -> usize {
                let len = std::cmp::min(buf.len(), self.buffer.len() - self.msg_len as usize);
                if len > 0 {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        std::mem::transmute(self.buffer.as_mut_ptr().offset(self.msg_len as _)),
                        len,
                    );
                    self.msg_len += len as u16; //The length is always less than 2^16.
                }
                len
            }
        }
        impl Write for $name {
            fn write_str(&mut self, s: &str) -> Result {
                unsafe {
                    self.write(s.as_bytes());
                }
                Ok(())
            }
        }
    };
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct SpanLog {
    buffer: [MaybeUninit<u8>; 512 - CTRL_LOG_SPAN], //TODO: fix
    id: NonZeroU32,
    duration_secs: u32,
    duration_nanos: u32,
    msg_len: u16,
}

impl_log_msg!(SpanLog);

impl SpanLog {
    pub fn new(id: NonZeroU32) -> SpanLog {
        SpanLog {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            id,
            msg_len: 0,
            duration_secs: 0,
            duration_nanos: 0,
        }
    }

    pub fn set_duration(&mut self, duration: &Duration) {
        self.duration_secs = duration.as_secs() as _;
        self.duration_nanos = duration.subsec_nanos()
    }

    pub fn get_duration(&self) -> Duration {
        Duration::new(self.duration_secs as _, self.duration_nanos)
    }

    pub fn clear(&mut self) {
        self.msg_len = 0;
    }

    pub fn id(&self) -> NonZeroU32 {
        self.id
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct EventLog {
    buffer: [MaybeUninit<u8>; 512 - CTRL_LOG_EVENT], //TODO: fix
    id: Option<NonZeroU32>,
    timestamp: i64,
    msg_len: u16,
    level: nt::header::Level,
}

impl_log_msg!(EventLog);

impl EventLog {
    pub fn new(id: Option<NonZeroU32>, timestamp: i64, level: nt::header::Level) -> EventLog {
        EventLog {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            id,
            timestamp,
            level,
            msg_len: 0,
        }
    }

    pub fn id(&self) -> Option<NonZeroU32> {
        self.id
    }

    pub fn level(&self) -> nt::header::Level {
        self.level
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
}
