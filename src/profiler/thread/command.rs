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

use std::time::Duration;
use bp3d_logger::LogMsg;
use crate::profiler::thread::util::FixedBufStr;
use crate::util::Meta;

#[derive(Clone, Debug)]
pub enum FixedBufValue {
    Float(f64),
    Signed(i64),
    Unsigned(u64),
    String(FixedBufStr<63>),
    Bool(bool),
}

#[derive(Clone)]
pub enum Command {
    Project {
        app_name: FixedBufStr<63>,
        name: FixedBufStr<63>,
        version: FixedBufStr<63>,
    },

    SpanAlloc {
        id: u64,
        metadata: Meta,
    },

    SpanInit {
        span: u64,
        parent: Option<u64>, //None must mean that span is at root
    },

    SpanFollows {
        span: u64,
        follows: u64,
    },

    SpanValue {
        span: u64,
        key: &'static str,
        value: FixedBufValue
    },

    SpanMessage {
        span: u64,
        message: FixedBufStr<255>
    },

    Event {
        id: u32,
        timestamp: i64,
        message: LogMsg
    },

    SpanEnter(u64),

    SpanExit {
        span: u64,
        duration: Duration,
    },

    SpanFree(u64),

    Terminate,
}
