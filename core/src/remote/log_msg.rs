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

use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::time::Duration;
use bp3d_debug::logger::Level;
use bp3d_debug::trace::span::Id;
use bp3d_debug::util::Location;
use crate::remote::network::common::{SpanId, SIZE_SPAN_ID};
use crate::remote::network::event::{Header, SIZE_HEADER};
use crate::remote::network::profiler::{RecordHeader, SIZE_RECORD_HEADER};

const BUFFER_LEN: usize = 512;
const CTRL_PROFILER_RECORD: usize = SIZE_RECORD_HEADER + size_of::<u16>() + 1;
const CTRL_FIELD_SET: usize = CTRL_PROFILER_RECORD;
const CTRL_EVENT: usize = size_of::<Location>() + SIZE_HEADER + size_of::<u16>() + 1;

pub trait Log: std::io::Write {
    fn increment_var_count(&mut self);
    unsafe fn write_single(&mut self, val: u8);

    //The right name for this function should be "write", but unfortunately should this function be named "write", it would be un-callable in Rust.
    unsafe fn write_multiple(&mut self, buf: &[u8]) -> usize;
}

macro_rules! impl_log_msg {
    ($name: ident) => {
        impl $name {
            pub fn as_bytes(&self) -> &[u8] {
                unsafe { std::mem::transmute(&self.buffer[..self.msg_len as usize]) }
            }
        }

        impl std::io::Write for $name {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                unsafe { Ok(self.write_multiple(buf)) }
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl Log for $name {
            fn increment_var_count(&mut self) {
                self.var_count += 1;
            }

            unsafe fn write_single(&mut self, val: u8) {
                let len = std::cmp::min(1, self.buffer.len() - self.msg_len as usize);
                if len > 0 {
                    self.buffer
                        .as_mut_ptr()
                        .offset(self.msg_len as _)
                        .write(MaybeUninit::new(val));
                    self.msg_len += 1;
                }
            }

            unsafe fn write_multiple(&mut self, buf: &[u8]) -> usize {
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
    };
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct ProfilerRecord {
    buffer: [MaybeUninit<u8>; BUFFER_LEN - CTRL_PROFILER_RECORD],
    header: RecordHeader<[u8; SIZE_RECORD_HEADER]>,
    msg_len: u16,
    var_count: u8,
}

impl_log_msg!(ProfilerRecord);

impl ProfilerRecord {
    pub fn new(id: NonZeroU32, start: u64, end: u64) -> ProfilerRecord {
        let mut header = RecordHeader::new_on_stack();
        header.set_id(id.get()).set_start(start).set_end(end);
        ProfilerRecord {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            header,
            msg_len: 0,
            var_count: 0,
        }
    }

    pub fn header(&self) -> RecordHeader<&[u8]> {
        self.header.to_ref()
    }

    pub fn var_count(&self) -> u8 {
        self.var_count
    }

    pub fn add_vars(&mut self, count: u8) {
        self.var_count += count;
    }

    pub fn get_duration(&self) -> Duration {
        let start = self.header.get_start();
        let end = self.header.get_end();
        let diff = end - start;
        Duration::from_nanos(diff)
    }

    pub fn clear(&mut self) {
        self.msg_len = 0;
        self.var_count = 0;
    }

    pub fn id(&self) -> NonZeroU32 {
        unsafe { NonZeroU32::new_unchecked(self.header.get_id()) }
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct FieldsetRecord {
    buffer: [MaybeUninit<u8>; BUFFER_LEN - CTRL_FIELD_SET],
    id: SpanId<[u8; SIZE_SPAN_ID]>,
    msg_len: u16,
    var_count: u8,
}

impl_log_msg!(FieldsetRecord);

impl FieldsetRecord {
    pub fn new() -> FieldsetRecord {
        FieldsetRecord {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            id: SpanId::new_on_stack(),
            msg_len: 0,
            var_count: 0,
        }
    }

    pub fn set_id(&mut self, id: Id) {
        self.id.set_callsite(id.get_callsite().get()).set_instance(id.get_instance().get());
    }

    pub fn var_count(&self) -> u8 {
        self.var_count
    }

    pub fn add_vars(&mut self, count: u8) {
        self.var_count += count;
    }

    pub fn clear(&mut self) {
        self.msg_len = 0;
        self.var_count = 0;
    }

    pub fn id(&self) -> SpanId<&[u8]> {
        self.id.to_ref()
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct EventLog {
    buffer: [MaybeUninit<u8>; BUFFER_LEN - CTRL_EVENT],
    location: Location,
    header: Header<[u8; SIZE_HEADER]>,
    msg_len: u16,
    var_count: u8,
}

impl_log_msg!(EventLog);

impl EventLog {
    pub fn new(
        id: Option<Id>,
        timestamp: i64,
        level: Level,
        location: Location,
    ) -> EventLog {
        let mut header = Header::new_on_stack();
        if let Some(id) = id {
            header.get_id_mut().set_callsite(id.get_callsite().get()).set_instance(id.get_instance().get());
        }
        header.set_timestamp(timestamp).set_raw_level(level as u8);
        EventLog {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            location,
            header,
            msg_len: 1,
            var_count: 0,
        }
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn var_count(&self) -> u8 {
        self.var_count
    }

    pub fn add_vars(&mut self, count: u8) {
        self.var_count += count;
    }

    pub fn header(&self) -> Header<&[u8]> {
        self.header.to_ref()
    }
}
