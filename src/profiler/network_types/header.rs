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

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[repr(u8)]
pub enum MsgType {
    Project = 0,
    SpanAlloc = 1,
    SpanParent = 2,
    SpanFollows = 3,
    SpanEvent = 4,
    SpanUpdate = 5,
    SpanDataset = 6
}

pub trait MsgSize {
    const SIZE: usize;
}

pub trait MsgHeader: MsgSize {
    const TYPE: MsgType;
    const HAS_PAYLOAD: bool;
}

impl<T: MsgSize> MsgSize for Option<T> {
    const SIZE: usize = T::SIZE + 1;
}

impl MsgSize for u32 {
    const SIZE: usize = 4;
}

#[derive(Serialize, Copy, Clone, Debug)]
pub struct PayloadRef {
    pub length: u16,
    pub offset: u16
}

impl MsgSize for PayloadRef {
    const SIZE: usize = 4;
}

pub type Vchar = PayloadRef;

#[derive(Serialize, Clone, Debug)]
pub struct Duration {
    pub seconds: u32,
    pub nano_seconds: u32
}

impl MsgSize for Duration {
    const SIZE: usize = 8;
}

impl From<&std::time::Duration> for Duration {
    fn from(value: &std::time::Duration) -> Self {
        Self {
            seconds: value.as_secs() as _,
            nano_seconds: value.subsec_nanos()
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
#[repr(u8)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4
}

impl MsgSize for Level {
    const SIZE: usize = 1;
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

impl MsgSize for Metadata {
    const SIZE: usize = Level::SIZE + Option::<u32>::SIZE + Vchar::SIZE * 2 + Option::<Vchar>::SIZE * 2;
}

#[derive(Serialize)]
pub struct Target {
    pub os: Vchar,
    pub family: Vchar,
    pub arch: Vchar
}

impl MsgSize for Target {
    const SIZE: usize = Vchar::SIZE * 3;
}

#[derive(Serialize)]
pub struct Cpu {
    pub name: Vchar,
    pub core_count: u32
}

impl MsgSize for Cpu {
    const SIZE: usize = Vchar::SIZE + 4;
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

impl MsgSize for Project {
    const SIZE: usize = Vchar::SIZE * 4 + Target::SIZE + Option::<Cpu>::SIZE;
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

impl MsgSize for SpanAlloc {
    const SIZE: usize = 4 + Metadata::SIZE;
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

impl MsgSize for SpanParent {
    const SIZE: usize = 8;
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

impl MsgSize for SpanFollows {
    const SIZE: usize = 8;
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

impl MsgSize for SpanEvent {
    const SIZE: usize = 4 + 8 + Level::SIZE + Vchar::SIZE;
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

impl MsgSize for SpanUpdate {
    const SIZE: usize = 8 + Duration::SIZE * 3;
}

impl MsgHeader for SpanUpdate {
    const TYPE: MsgType = MsgType::SpanUpdate;
    const HAS_PAYLOAD: bool = false;
}

#[derive(Serialize)]
pub struct SpanDataset {
    pub id: u32,
    pub run_count: u32,
    pub size: u32
}

impl MsgSize for SpanDataset {
    const SIZE: usize = u32::SIZE * 3;
}

impl MsgHeader for SpanDataset {
    const TYPE: MsgType = MsgType::SpanDataset;

    const HAS_PAYLOAD: bool = true;
}

#[derive(Deserialize)]
pub struct ClientRecord {
    pub max_rows: u32,
    pub enable: bool
}

impl MsgSize for ClientRecord {
    const SIZE: usize = u32::SIZE + 1;
}

#[derive(Deserialize)]
pub struct ClientConfig {
    pub max_average_points: u32,
    pub max_level: Option<Level>,
    pub record: ClientRecord,
    pub period: u16
}

impl MsgSize for ClientConfig {
    const SIZE: usize = 9 + Option::<Level>::SIZE;
}

#[derive(Serialize)]
pub struct ServerConfig {
    pub max_rows: u32
}

impl MsgSize for ServerConfig {
    const SIZE: usize = Option::<Vchar>::SIZE;
}
