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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Float(f64),
    Signed(i64),
    Unsigned(u64),
    String(String),
    Bool(bool)
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Serialize, Deserialize)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error
}

impl Level {
    pub fn from_tracing(level: tracing::Level) -> Level {
        match level {
            tracing::Level::TRACE => Level::Trace,
            tracing::Level::DEBUG => Level::Debug,
            tracing::Level::INFO => Level::Info,
            tracing::Level::WARN => Level::Warning,
            tracing::Level::ERROR => Level::Error
        }
    }
    pub fn from_log(level: log::Level) -> Level {
        match level {
            log::Level::Trace => Level::Trace,
            log::Level::Debug => Level::Debug,
            log::Level::Info => Level::Info,
            log::Level::Warn => Level::Warning,
            log::Level::Error => Level::Error
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Metadata {
    name: String, //The name of the span/event
    target: String, //The target of the span/event (usually this contains module path)
    level: Level, //The log level of the span/event
    module_path: Option<String>, //The module path (including crate name)
    file: Option<String>, //The file path
    line: Option<u32> //The line number in the file
}

impl Metadata {
    pub fn from_log(meta: &log::Record) -> Metadata {
        Metadata {
            name: "<log>".into(),
            target: meta.target().into(),
            level: Level::from_log(meta.level()),
            module_path: meta.module_path().map(|v| v.into()),
            file: meta.file().map(|v| v.into()),
            line: meta.line()
        }
    }
    pub fn from_tracing(meta: &tracing::Metadata) -> Metadata {
        Metadata {
            name: meta.name().into(),
            target: meta.target().into(),
            level: Level::from_tracing(*meta.level()),
            module_path: meta.module_path().map(|v| v.into()),
            file: meta.file().map(|v| v.into()),
            line: meta.line()
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Command {
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
        duration: f64
    },

    SpanFree(SpanId),

    Terminate
}
