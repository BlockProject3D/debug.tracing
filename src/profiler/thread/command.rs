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

use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;
use bp3d_logger::LogMsg;
use crate::profiler::thread::util::FixedBufStr;
use crate::util::{Meta, SpanId};

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
        id: SpanId,
        metadata: Meta,
    },

    SpanInit {
        span: SpanId,
        parent: Option<SpanId>, //None must mean that span is at root
    },

    SpanFollows {
        span: SpanId,
        follows: SpanId,
    },

    SpanValue {
        span: SpanId,
        key: &'static str,
        value: FixedBufValue
    },

    SpanMessage {
        span: SpanId,
        message: FixedBufStr<255>
    },

    Event {
        id: NonZeroU32,
        timestamp: i64,
        message: LogMsg
    },

    SpanEnter(SpanId),

    SpanExit {
        span: SpanId,
        duration: Duration,
    },

    SpanFree(SpanId),

    Terminate,
}

#[derive(Debug)]
pub enum Control {
    Project {
        app_name: FixedBufStr<63>,
        name: FixedBufStr<63>,
        version: FixedBufStr<63>,
    },

    Terminate
}

#[derive(Debug)]
pub enum SpanControl {
    Alloc {
        metadata: Meta
    },

    /*Value {
        key: &'static str,
        value: FixedBufValue
    },

    Message {
        message: FixedBufStr<63>
    },

    Init {
        parent: Option<SpanId> //None must mean that span is at root
    },*/

    UpdateParent {
        parent: Option<NonZeroU32> //None must mean that span is at root
    },

    Follows {
        follows: SpanId
    },

    /*Exit {
        duration: Duration,
    },*/

    //Free
}

#[derive(Debug)]
pub enum SpanData {
    Value {
        key: &'static str,
        value: FixedBufValue
    },

    Message {
        message: FixedBufStr<63>
    }
}

#[derive(Debug)]
pub struct Span<T> {
    pub id: SpanId,
    pub ty: T
}

pub struct Event {
    pub id: Option<NonZeroU32>,
    pub timestamp: i64,
    pub message: LogMsg
}
