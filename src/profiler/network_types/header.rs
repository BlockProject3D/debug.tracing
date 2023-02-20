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

use serde::Serialize;

#[derive(Serialize)]
#[repr(u8)]
pub enum MsgType {
    Project = 0,
    SpanAlloc = 1,
    SpanParent = 2,
    SpanFollows = 3,
    SpanEvent = 4,
    SpanUpdate = 5
}

pub trait MsgHeader {
    const TYPE: MsgType;
    const HAS_PAYLOAD: bool;
}

#[derive(Serialize, Copy, Clone, Debug)]
pub struct PayloadRef {
    pub length: u16,
    pub offset: u16
}

pub type Vchar = PayloadRef;

#[derive(Serialize, Clone, Debug)]
pub struct Duration {
    pub seconds: u32,
    pub nano_seconds: u32
}

impl From<&std::time::Duration> for Duration {
    fn from(value: &std::time::Duration) -> Self {
        Self {
            seconds: value.as_secs() as _,
            nano_seconds: value.subsec_nanos()
        }
    }
}

#[derive(Serialize, Copy, Clone, Debug)]
#[repr(u8)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4
}

impl Level {
    pub fn from_tracing(level: tracing::Level) -> Level {
        match level {
            tracing::Level::TRACE => Level::Trace,
            tracing::Level::DEBUG => Level::Debug,
            tracing::Level::WARN => Level::Warning,
            tracing::Level::ERROR => Level::Error,
            _ => Level::Info
        }
    }

    pub fn from_log(level: log::Level) -> Level {
        match level {
            log::Level::Trace => Level::Trace,
            log::Level::Debug => Level::Debug,
            log::Level::Warn => Level::Warning,
            log::Level::Error => Level::Error,
            _ => Level::Info
        }
    }
}

#[derive(Serialize)]
pub struct Metadata {
    pub level: Level,
    pub line: Option<u32>,
    pub name: Vchar,
    pub target: Vchar,
    pub module_path: Option<Vchar>,
    pub file: Option<Vchar>
}

#[derive(Serialize)]
pub struct Target {
    pub os: Vchar,
    pub family: Vchar,
    pub arch: Vchar
}

#[derive(Serialize)]
pub struct Cpu {
    pub name: Vchar,
    pub core_count: u32
}

#[derive(Serialize)]
pub struct Project {
    pub app_name: Vchar,
    pub name: Vchar,
    pub version: Vchar,
    pub cmd_line: Vchar,
    pub target: Target,
    pub cpu: Option<Cpu>
}

impl MsgHeader for Project {
    const TYPE: MsgType = MsgType::Project;
    const HAS_PAYLOAD: bool = true;
}

#[derive(Serialize)]
pub struct SpanAlloc {
    pub id: u32,
    pub metadata: Metadata
}

impl MsgHeader for SpanAlloc {
    const TYPE: MsgType = MsgType::SpanAlloc;
    const HAS_PAYLOAD: bool = true;
}

#[derive(Serialize)]
pub struct SpanParent {
    pub id: u32,
    pub parent_node: u32 //0 = No parent
}

impl MsgHeader for SpanParent {
    const TYPE: MsgType = MsgType::SpanParent;
    const HAS_PAYLOAD: bool = false;
}

#[derive(Serialize)]
pub struct SpanFollows {
    pub id: u32,
    pub follows: u32
}

impl MsgHeader for SpanFollows {
    const TYPE: MsgType = MsgType::SpanFollows;
    const HAS_PAYLOAD: bool = false;
}

#[derive(Serialize)]
pub struct SpanEvent {
    pub id: u32,
    pub timestamp: i64,
    pub level: Level,
    pub message: Vchar
}

impl MsgHeader for SpanEvent {
    const TYPE: MsgType = MsgType::SpanEvent;
    const HAS_PAYLOAD: bool = true;
}

#[derive(Serialize)]
pub struct SpanUpdate {
    pub id: u32,
    pub run_count: u32,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration
}

impl MsgHeader for SpanUpdate {
    const TYPE: MsgType = MsgType::SpanUpdate;
    const HAS_PAYLOAD: bool = false;
}
