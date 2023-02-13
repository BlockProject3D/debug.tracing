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
use std::io::{Cursor, Seek, SeekFrom, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::profiler::network_types::util::{DebugWriter, Payload, WriteInto};

pub enum Value<'a> {
    Float(f64),
    Signed(i64),
    Unsigned(u64),
    String(&'a str),
    Bool(bool),
    Debug(&'a dyn Debug)
}

pub struct Variable<'a> {
    pub name: &'a str,
    pub value: Value<'a>
}

impl<'a> Variable<'a> {
    pub fn from_buffer(buf: &mut Cursor<&'a [u8]>) -> Option<Variable<'a>> {
        let name_len = buf.read_u8().ok()?;
        let name_start = buf.stream_position().ok()?;
        let name_end = buf.seek(SeekFrom::Current(name_len as _)).ok()?;
        let name = std::str::from_utf8(&buf.get_ref()[name_start as _..name_end as _]).ok()?;
        let ty = buf.read_u8().ok()?;
        match ty {
            0 => {
                // Read float
                let val = buf.read_f64::<LittleEndian>().ok()?;
                Some(Variable {
                    name,
                    value: Value::Float(val)
                })
            },
            1 => {
                // Read signed
                let val = buf.read_i64::<LittleEndian>().ok()?;
                Some(Variable {
                    name,
                    value: Value::Signed(val)
                })
            },
            2 => {
                // Read unsigned
                let val = buf.read_u64::<LittleEndian>().ok()?;
                Some(Variable {
                    name,
                    value: Value::Unsigned(val)
                })
            },
            4 => {
                // Read bool
                let val = buf.read_u8().ok()?;
                match val {
                    1 => Some(Variable {
                        name,
                        value: Value::Bool(true)
                    }),
                    _ => Some(Variable {
                        name,
                        value: Value::Bool(false)
                    })
                }
            },
            _ => {
                // Read string
                let str_len = buf.read_u8().ok()?;
                let str_start = buf.stream_position().ok()?;
                let str_end = buf.seek(SeekFrom::Current(str_len as _)).ok()?;
                let str = std::str::from_utf8(&buf.get_ref()[str_start as _..str_end as _]).ok()?;
                Some(Variable {
                    name,
                    value: Value::String(str)
                })
            }
        }
    }
}

impl WriteInto for str {
    fn write_into(&self, buf: &mut Payload) -> std::io::Result<()> {
        buf.write(self.as_bytes()).map(|_| ())
    }
}

impl<'a> WriteInto for Variable<'a> {
    fn write_into(&self, buf: &mut Payload) -> std::io::Result<()> {
        let name_bytes = self.name.as_bytes();
        buf.write_u8(name_bytes.len() as _)?;
        buf.write(name_bytes)?;
        match self.value {
            Value::Float(v) => {
                buf.write_u8(0)?;
                buf.write_f64::<LittleEndian>(v)
            },
            Value::Signed(v) => {
                buf.write_u8(1)?;
                buf.write_i64::<LittleEndian>(v)
            },
            Value::Unsigned(v) => {
                buf.write_u8(2)?;
                buf.write_u64::<LittleEndian>(v)
            },
            Value::String(v) => {
                buf.write_u8(3)?;
                buf.write_u8(v.as_bytes().len() as _)?;
                v.write_into(buf)
            },
            Value::Bool(v) => {
                buf.write_u8(4)?;
                if v {
                    buf.write_u8(1)
                } else {
                    buf.write_u8(0)
                }
            },
            Value::Debug(v) => {
                buf.write_u8(3)?;
                let pos = buf.stream_position()?;
                buf.write_u8(0)?;
                let r = buf.write_object(&DebugWriter(v))?;
                buf.seek(SeekFrom::Start(pos))?;
                buf.write_u8(r.length as _)?;
                buf.seek(SeekFrom::Current(r.length as _)).map(|_| ())
            }
        }
    }
}
