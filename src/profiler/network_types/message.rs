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
    SpanDataset = 6,
    ServerConfig = 7,
}

pub trait MsgSize {
    const SIZE: usize;
}

pub trait Msg {
    const TYPE: MsgType;
    const HAS_PAYLOAD: bool;
}

impl<T: MsgSize> MsgSize for Option<T> {
    const SIZE: usize = T::SIZE + 1;
}

impl MsgSize for u32 {
    const SIZE: usize = 4;
}

impl MsgSize for u16 {
    const SIZE: usize = 2;
}

#[derive(Serialize, Clone, Debug)]
pub struct Duration {
    pub seconds: u32,
    pub nano_seconds: u32,
}

impl MsgSize for Duration {
    const SIZE: usize = 8;
}

impl From<&std::time::Duration> for Duration {
    fn from(value: &std::time::Duration) -> Self {
        Self {
            seconds: value.as_secs() as _,
            nano_seconds: value.subsec_nanos(),
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
    Error = 4,
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
            _ => Level::Info,
        }
    }

    pub fn from_log(level: log::Level) -> Level {
        match level {
            log::Level::Trace => Level::Trace,
            log::Level::Debug => Level::Debug,
            log::Level::Warn => Level::Warning,
            log::Level::Error => Level::Error,
            _ => Level::Info,
        }
    }
}

#[derive(Serialize)]
pub struct Metadata<'a> {
    pub level: Level,
    pub line: Option<u32>,
    pub name: &'a str,
    pub target: &'a str,
    pub module_path: Option<&'a str>,
    pub file: Option<&'a str>,
}

#[derive(Serialize)]
pub struct Target<'a> {
    pub os: &'a str,
    pub family: &'a str,
    pub arch: &'a str,
}

#[derive(Serialize)]
pub struct Cpu {
    pub name: String,
    pub core_count: u32,
}

#[derive(Serialize)]
pub struct Project<'a> {
    pub app_name: &'a str,
    pub name: &'a str,
    pub version: &'a str,
    pub cmd_line: &'a str,
    pub target: Target<'a>,
    pub cpu: Option<Cpu>,
}

impl<'a> Msg for Project<'a> {
    const TYPE: MsgType = MsgType::Project;
    const HAS_PAYLOAD: bool = true;
}

#[derive(Serialize)]
pub struct SpanAlloc<'a> {
    pub id: u32,
    pub metadata: Metadata<'a>,
}

impl<'a> Msg for SpanAlloc<'a> {
    const TYPE: MsgType = MsgType::SpanAlloc;
    const HAS_PAYLOAD: bool = true;
}

#[derive(Serialize)]
pub struct SpanParent {
    pub id: u32,
    pub parent_node: u32, //0 = No parent
}

impl MsgSize for SpanParent {
    const SIZE: usize = 8;
}

impl Msg for SpanParent {
    const TYPE: MsgType = MsgType::SpanParent;
    const HAS_PAYLOAD: bool = false;
}

#[derive(Serialize)]
pub struct SpanFollows {
    pub id: u32,
    pub follows: u32,
}

impl MsgSize for SpanFollows {
    const SIZE: usize = 8;
}

impl Msg for SpanFollows {
    const TYPE: MsgType = MsgType::SpanFollows;
    const HAS_PAYLOAD: bool = false;
}

#[derive(Serialize)]
pub struct SpanEvent<'a> {
    pub id: u32,
    pub timestamp: i64,
    pub level: Level,
    pub message: &'a [u8],
}

impl<'a> Msg for SpanEvent<'a> {
    const TYPE: MsgType = MsgType::SpanEvent;
    const HAS_PAYLOAD: bool = true;
}

#[derive(Serialize)]
pub struct SpanUpdate {
    pub id: u32,
    pub run_count: u32,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
}

impl MsgSize for SpanUpdate {
    const SIZE: usize = 8 + Duration::SIZE * 3;
}

impl Msg for SpanUpdate {
    const TYPE: MsgType = MsgType::SpanUpdate;
    const HAS_PAYLOAD: bool = false;
}

#[derive(Serialize)]
pub struct SpanDataset {
    pub id: u32,
    pub run_count: u32
}

impl MsgSize for SpanDataset {
    const SIZE: usize = u32::SIZE * 2;
}

impl<'a> Msg for SpanDataset {
    const TYPE: MsgType = MsgType::SpanDataset;

    const HAS_PAYLOAD: bool = true;
}

#[derive(Deserialize, Default)]
pub struct ClientRecord {
    pub max_rows: u32,
    pub enable: bool,
}

impl MsgSize for ClientRecord {
    const SIZE: usize = u32::SIZE + 1;
}

#[derive(Deserialize, Default)]
pub struct ClientConfig {
    pub max_average_points: u32,
    pub max_level: Option<Level>,
    pub record: ClientRecord,
    pub period: u16,
}

impl MsgSize for ClientConfig {
    const SIZE: usize = u32::SIZE + Option::<Level>::SIZE + ClientRecord::SIZE + 2;
}

#[derive(Serialize)]
pub struct ServerConfig {
    pub max_rows: u32,
    pub min_period: u16,
}

impl MsgSize for ServerConfig {
    const SIZE: usize = u32::SIZE + u16::SIZE;
}

impl Msg for ServerConfig {
    const TYPE: MsgType = MsgType::ServerConfig;

    const HAS_PAYLOAD: bool = false;
}
