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
use std::fmt::{Debug, Write};
use std::mem::MaybeUninit;

#[derive(Clone, Debug)]
pub struct FixedBufStr<const N: usize> {
    buffer: [MaybeUninit<u8>; N],
    len: usize
}

impl<const N: usize> FixedBufStr<N> {
    pub fn new() -> FixedBufStr<N> {
        FixedBufStr {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0
        }
    }

    pub fn str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(std::mem::transmute(&self.buffer[..self.len]))
        }
    }

    pub fn from_str(value: &str) -> Self {
        let mut buffer = FixedBufStr::new();
        let len = std::cmp::min(value.len(), N);
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr(), std::mem::transmute(buffer.buffer.as_mut_ptr()), len);
        }
        buffer.len = len;
        buffer
    }

    pub fn from_debug<T: Debug>(value: T) -> Self {
        let mut buffer = FixedBufStr::new();
        let _ = write!(buffer, "{:?}", value);
        buffer
    }
}

impl<const N: usize> Write for FixedBufStr<N> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let len = std::cmp::min(value.len(), N);
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr(), std::mem::transmute(self.buffer.as_mut_ptr()), len);
        }
        self.len = len;
        Ok(())
    }
}

pub fn read_command_line(payload: &mut nt::util::Payload) -> nt::header::Vchar {
    let mut r = payload.write_object("").unwrap();
    for v in std::env::args_os() {
        let head = payload.write_object(&*v.to_string_lossy()).unwrap();
        use std::io::Write;
        payload.write(b" ").unwrap();
        r.length += head.length + 1;
    }
    r
}
