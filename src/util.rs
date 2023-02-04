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

use tracing_core::span::Id;
use tracing_core::{Level, Metadata};

pub type Meta = &'static Metadata<'static>;

pub fn hash_static_ref<T: ?Sized>(meta: &'static T) -> usize {
    let ptr = meta as *const T;
    ptr as *const () as usize
}

pub fn extract_target_module<'a>(record: Meta) -> (&'a str, Option<&'a str>) {
    let base_string = record.module_path().unwrap_or_else(|| record.target());
    let target = base_string
        .find("::")
        .map(|v| &base_string[..v])
        .unwrap_or(base_string);
    let module = base_string.find("::").map(|v| &base_string[(v + 2)..]);
    (target, module)
}

pub fn tracing_level_to_log(level: &Level) -> log::Level {
    match *level {
        Level::TRACE => log::Level::Trace,
        Level::DEBUG => log::Level::Debug,
        Level::INFO => log::Level::Info,
        Level::WARN => log::Level::Warn,
        Level::ERROR => log::Level::Error,
    }
}

pub fn span_from_id_instance(span_id: u32, instance: u32) -> Id {
    Id::from_u64((span_id as u64) << 32 | instance as u64)
}

pub fn span_to_id_instance(span: &Id) -> (u32, u32) {
    let combined = span.into_u64();
    ((combined >> 32) as u32, combined as u32)
}
