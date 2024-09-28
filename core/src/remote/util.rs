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

use std::fmt::{Debug, Display};
use std::io::Write;
use bp3d_debug::field::{Field, FieldValue};
use crate::remote::network as net;
use bp3d_proto::message::{WriteSelf, WriteTo};
use bp3d_proto::util::Wrap;

pub trait WriteField {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], out: W);
}

impl WriteField for u64 {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], out: W) {
        let (header_buffer, data_buffer) = buffer.split_at_mut(net::value::SIZE_HEADER);
        let mut header = net::value::Header::wrap(header_buffer);
        if *self < 256 {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::UInt8).to_ref(),
                name,
                value: net::value::UInt8::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        } else if *self < 65536 {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::UInt16).to_ref(),
                name,
                value: net::value::UInt16::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        } else if *self < u32::MAX as u64 + 1 {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::UInt32).to_ref(),
                name,
                value: net::value::UInt32::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        } else {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::UInt64).to_ref(),
                name,
                value: net::value::UInt64::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        }
    }
}

impl WriteField for i64 {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], out: W) {
        let (header_buffer, data_buffer) = buffer.split_at_mut(net::value::SIZE_HEADER);
        let mut header = net::value::Header::wrap(header_buffer);
        if *self < 128 {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::Int8).to_ref(),
                name,
                value: net::value::Int8::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        } else if *self < 32768 {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::Int16).to_ref(),
                name,
                value: net::value::Int16::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        } else if *self < i32::MAX as i64 + 1 {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::Int32).to_ref(),
                name,
                value: net::value::Int32::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        } else {
            let _ = net::common::Field {
                header: header.set_type(net::value::Type::Int64).to_ref(),
                name,
                value: net::value::Int64::wrap(data_buffer).set_data(*self as _).to_ref()
            }.write_self(out);
        }
    }
}

impl<D: Display> WriteField for &D {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], mut out: W) {
        let _ = bp3d_proto::message::util::NullTerminatedString::write_to(&name, &mut out);
        let _ = net::value::Header::wrap(buffer).set_type(net::value::Type::String).to_ref().write_self(&mut out);
        let _ = write!(out, "{}", self);
        let _ = out.write(&[0]);
    }
}

impl WriteField for &dyn Debug {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], mut out: W) {
        let _ = bp3d_proto::message::util::NullTerminatedString::write_to(&name, &mut out);
        let _ = net::value::Header::wrap(buffer).set_type(net::value::Type::String).to_ref().write_self(&mut out);
        let _ = write!(out, "{:?}", self);
        let _ = out.write(&[0]);
    }
}

impl WriteField for &str {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], out: W) {
        let _ = net::common::Field {
            header: net::value::Header::wrap(buffer).set_type(net::value::Type::String).to_ref(),
            name,
            value: net::value::String { data: self }
        }.write_self(out);
    }
}

impl WriteField for f64 {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], out: W) {
        let (header_buffer, data_buffer) = buffer.split_at_mut(net::value::SIZE_HEADER);
        let _ = net::common::Field {
            header: net::value::Header::wrap(header_buffer).set_type(net::value::Type::Double).to_ref(),
            name,
            value: net::value::Double::wrap(data_buffer).set_data(*self).to_ref()
        }.write_self(out);
    }
}

impl WriteField for bool {
    fn write_field<W: Write>(&self, name: &str, buffer: &mut [u8], out: W) {
        let (header_buffer, data_buffer) = buffer.split_at_mut(net::value::SIZE_HEADER);
        let _ = net::common::Field {
            header: net::value::Header::wrap(header_buffer).set_type(net::value::Type::Bool).to_ref(),
            name,
            value: net::value::Bool::wrap(data_buffer).set_data(*self).to_ref()
        }.write_self(out);
    }
}

pub fn write_fields(fields: &[Field], mut out: impl Write) {
    let mut buffer = [0; net::value::SIZE_HEADER + net::value::SIZE_U_INT64];
    for field in fields {
        match field.value() {
            FieldValue::Int(v) => v.write_field(field.name(), &mut buffer, &mut out),
            FieldValue::UInt(v) => v.write_field(field.name(), &mut buffer, &mut out),
            FieldValue::Float(v) => v.write_field(field.name(), &mut buffer, &mut out),
            FieldValue::Double(v) => v.write_field(field.name(), &mut buffer, &mut out),
            FieldValue::String(v) => v.write_field(field.name(), &mut buffer, &mut out),
            FieldValue::Debug(v) => v.write_field(field.name(), &mut buffer, &mut out)
        }
    }
}
