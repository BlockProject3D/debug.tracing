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

use serde::{Serialize, Deserialize};
use tracing_core::span::Id;
use crate::profiler::network_types::{Metadata, Value};
use crate::util::span_to_id_instance;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanId {
    id: u32,
    instance: u32
}

impl SpanId {
    pub fn from_u64(span: u64) -> SpanId {
        let (id, instance) = span_to_id_instance(&Id::from_u64(span));
        SpanId {
            id,
            instance
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub name: String,
    pub core_count: u32
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    pub os: String,
    pub family: String,
    pub arch: String
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Duration {
    pub seconds: u64,
    pub nano_seconds: u32
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Command {
    Project {
        app_name: String,
        name: String,
        version: String,
        target: TargetInfo,
        command_line: String,
        cpu: Option<CpuInfo>
    },

    SpanAlloc {
        id: SpanId,
        metadata: Metadata
    },

    SpanInit {
        span: SpanId,
        parent: Option<SpanId>, //None must mean that span is at root
        message: Option<String>,
        value_set: Vec<(String, Value)>
    },

    SpanFollows {
        span: SpanId,
        follows: SpanId
    },

    SpanValues {
        span: SpanId,
        message: Option<String>,
        value_set: Vec<(String, Value)>
    },

    Event {
        span: Option<SpanId>,
        metadata: Metadata,
        time: i64,
        message: Option<String>,
        value_set: Vec<(String, Value)>
    },

    SpanEnter(SpanId),

    SpanExit {
        span: SpanId,
        duration: Duration
    },

    SpanFree(SpanId),

    Terminate
}
