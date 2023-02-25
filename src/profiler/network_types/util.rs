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

use std::fmt::Debug;
use std::io::{Seek, SeekFrom, Write};
use crate::profiler::network_types::header::PayloadRef;

pub struct Payload<'a> {
    buffer: &'a mut [u8],
    cursor: &'a mut usize
}

impl<'a> Payload<'a> {
    pub fn new(buffer: &'a mut [u8], cursor: &'a mut usize) -> Payload<'a> {
        Payload {
            buffer,
            cursor
        }
    }

    pub fn write_object<T: WriteInto + ?Sized>(&mut self, obj: &T) -> std::io::Result<PayloadRef> {
        let start = *self.cursor;
        obj.write_into(self)?;
        Ok(PayloadRef {
            offset: start as _,
            length: (*self.cursor - start) as _
        })
    }
}

impl<'a> Write for Payload<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = std::cmp::min(buf.len(), self.buffer.len() - *self.cursor);
        self.buffer[*self.cursor..*self.cursor + len].copy_from_slice(&buf[..len]);
        *self.cursor += len;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.write(buf).map(|_| ())
    }
}

impl<'a> Seek for Payload<'a> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(v) => {
                if v < self.buffer.len() as _ {
                    *self.cursor = v as _;
                }
                Ok(v)
            }
            SeekFrom::End(v) => {
                let pos = self.buffer.len().wrapping_add(v as _);
                if pos < self.buffer.len() {
                    *self.cursor = pos;
                }
                Ok(*self.cursor as _)
            }
            SeekFrom::Current(v) => {
                let pos = self.cursor.wrapping_add(v as _);
                if pos < self.buffer.len() {
                    *self.cursor = pos;
                }
                Ok(*self.cursor as _)
            }
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(*self.cursor as _)
    }
}

pub struct DebugWriter<'a>(pub &'a dyn Debug);

pub trait WriteInto {
    fn write_into(&self, buf: &mut Payload) -> std::io::Result<()>;
}

impl<'a> WriteInto for DebugWriter<'a> {
    fn write_into(&self, buf: &mut Payload) -> std::io::Result<()> {
        write!(buf, "{:?}", self.0)
    }
}
