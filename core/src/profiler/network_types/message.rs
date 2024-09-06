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

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[repr(u8)]
pub enum MsgType {
    Project = 0,
    ProfilerSectionRegister = 1,
    Event = 4,
    ProfilerSectionUpdate = 5,
    ProfilerDataset = 6,
    ServerConfig = 7,
    SpanAlloc = 8,
    SpanEnter = 9,
    SpanExit = 10
}

pub trait MsgSize {
    const SIZE: usize;
}

pub trait Msg {
    const TYPE: MsgType;
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

impl MsgSize for i64 {
    const SIZE: usize = 8;
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

impl From<bp3d_debug::logger::Level> for Level {
    fn from(value: bp3d_debug::logger::Level) -> Self {
        unsafe { std::mem::transmute(value as u8 - 1) }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
#[repr(u8)]
pub enum ProfilerSectionLevel {
    Critical = 0,
    Periodic = 1,
    Event = 2
}

impl MsgSize for ProfilerSectionLevel {
    const SIZE: usize = 1;
}

impl From<bp3d_debug::profiler::section::Level> for ProfilerSectionLevel {
    fn from(value: bp3d_debug::profiler::section::Level) -> Self {
        unsafe { std::mem::transmute(value as u8) }
    }
}

#[derive(Serialize)]
pub struct ProfilerSectionCallsite<'a> {
    pub level: ProfilerSectionLevel,
    pub line: u32,
    pub name: &'a str,
    pub module_path: &'a str,
    pub file: &'a str,
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
}

#[derive(Serialize)]
pub struct ProfilerSectionRegister<'a> {
    pub id: u32,
    pub metadata: ProfilerSectionCallsite<'a>,
}

impl<'a> Msg for ProfilerSectionRegister<'a> {
    const TYPE: MsgType = MsgType::ProfilerSectionRegister;
}

#[derive(Serialize)]
pub struct Event {
    pub id: u32,
    pub timestamp: i64,
    pub level: Level,
}

impl Msg for Event {
    const TYPE: MsgType = MsgType::Event;
}

impl MsgSize for Event {
    const SIZE: usize = u32::SIZE + i64::SIZE + Level::SIZE;
}

#[derive(Serialize)]
pub struct ProfilerSectionUpdate {
    pub id: u32,
    pub run_count: u32,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
}

impl MsgSize for ProfilerSectionUpdate {
    const SIZE: usize = 8 + Duration::SIZE * 3;
}

impl Msg for ProfilerSectionUpdate {
    const TYPE: MsgType = MsgType::ProfilerSectionUpdate;
}

#[derive(Serialize)]
pub struct ProfilerDataset {
    pub id: u32,
    pub run_count: u32,
}

impl MsgSize for ProfilerDataset {
    const SIZE: usize = u32::SIZE * 2;
}

impl<'a> Msg for ProfilerDataset {
    const TYPE: MsgType = MsgType::ProfilerDataset;
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
}
