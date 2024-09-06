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

use bp3d_debug::trace::span::Id;
use std::fmt::Write;

impl<T: AsMut<[u8]>> crate::profiler::network::common::Duration<T> {
    pub fn from_std(&mut self, value: &std::time::Duration) -> &mut Self {
        self.set_seconds(value.as_secs() as _)
            .set_nano_seconds(value.subsec_nanos());
        self
    }
}

impl SpanId<[u8; SIZE_SPAN_ID]> {
    pub fn from_debug(value: Id) -> Self {
        let mut val = SpanId::new_on_stack();
        val.set_callsite(value.get_callsite().get())
            .set_instance(value.get_instance().get());
        val
    }
}

pub fn read_command_line<W: Write>(write: &mut W) {
    for v in std::env::args_os() {
        let _ = write!(write, "{} ", v.to_string_lossy());
    }
}

macro_rules! wrap_io_debug_error {
    ($e: expr) => {
        if let Err(e) = $e {
            eprintln!("Failed to write to network: {}", e);
        }
    };
}

use crate::profiler::network::common::{SpanId, SIZE_SPAN_ID};
pub(crate) use wrap_io_debug_error;
