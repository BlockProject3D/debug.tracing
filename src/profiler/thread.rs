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

use crate::profiler::cpu_info::read_cpu_info;
use crate::profiler::network_types::Command as NetCommand;
use crate::profiler::network_types::{Duration, Metadata, Project, SpanId, TargetInfo, Value};
use crate::util::Meta;
use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::Receiver;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::net::TcpStream;

#[derive(Debug, Clone)]
pub enum Event {
    Borrowed {
        span: Option<u64>,
        metadata: Meta,
        time: i64,
        message: Option<String>,
        value_set: Vec<(&'static str, Value)>,
    },
    Owned {
        span: Option<u64>,
        metadata: Metadata,
        time: i64,
        message: Option<String>,
        value_set: Vec<(&'static str, Value)>,
    },
}

#[derive(Clone, Debug)]
pub enum Command {
    Project {
        app_name: String,
        name: String,
        version: String,
    },

    SpanAlloc {
        id: u64,
        metadata: Meta,
    },

    SpanInit {
        span: u64,
        parent: Option<u64>, //None must mean that span is at root
        message: Option<String>,
        value_set: Vec<(&'static str, Value)>,
    },

    SpanFollows {
        span: u64,
        follows: u64,
    },

    SpanValues {
        span: u64,
        message: Option<String>,
        value_set: Vec<(&'static str, Value)>,
    },

    Event(Event),

    SpanEnter(u64),

    SpanExit {
        span: u64,
        duration: Duration,
    },

    SpanFree(u64),

    Terminate,
}

fn get_command_line() -> String {
    std::env::args_os()
        .collect::<Vec<OsString>>()
        .join(OsStr::new(" "))
        .to_string_lossy()
        .into()
}

impl Command {
    pub fn to_network(self) -> super::network_types::Command {
        use super::network_types::Metadata as NetMeta;
        match self {
            Command::SpanAlloc { id, metadata } => NetCommand::SpanAlloc {
                id: SpanId::from_u64(id),
                metadata: NetMeta::from_tracing(metadata),
            },
            Command::SpanInit {
                span,
                parent,
                message,
                value_set,
            } => NetCommand::SpanInit {
                span: SpanId::from_u64(span),
                parent: parent.map(SpanId::from_u64),
                message,
                value_set: value_set.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            },
            Command::SpanFollows { span, follows } => NetCommand::SpanFollows {
                span: SpanId::from_u64(span),
                follows: SpanId::from_u64(follows),
            },
            Command::SpanValues {
                span,
                message,
                value_set,
            } => NetCommand::SpanValues {
                span: SpanId::from_u64(span),
                message,
                value_set: value_set.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            },
            Command::Event(ev) => match ev {
                Event::Borrowed {
                    span,
                    metadata,
                    time,
                    message,
                    value_set,
                } => NetCommand::Event {
                    span: span.map(SpanId::from_u64),
                    metadata: NetMeta::from_tracing(metadata),
                    time,
                    message,
                    value_set: value_set.into_iter().map(|(k, v)| (k.into(), v)).collect(),
                },
                Event::Owned {
                    span,
                    metadata,
                    time,
                    message,
                    value_set,
                } => NetCommand::Event {
                    span: span.map(SpanId::from_u64),
                    metadata,
                    time,
                    message,
                    value_set: value_set.into_iter().map(|(k, v)| (k.into(), v)).collect(),
                },
            },
            Command::SpanEnter(v) => NetCommand::SpanEnter(SpanId::from_u64(v)),
            Command::SpanExit { span, duration } => NetCommand::SpanExit {
                span: SpanId::from_u64(span),
                duration,
            },
            Command::SpanFree(v) => NetCommand::SpanFree(SpanId::from_u64(v)),
            Command::Project {
                app_name,
                name,
                version,
            } => NetCommand::Project(Project {
                app_name,
                name,
                version,
                target: TargetInfo {
                    os: std::env::consts::OS.into(),
                    family: std::env::consts::FAMILY.into(),
                    arch: std::env::consts::ARCH.into(),
                },
                command_line: get_command_line(),
                cpu: read_cpu_info(),
            }),
            Command::Terminate => NetCommand::Terminate,
        }
    }
}

pub struct Thread {
    socket: TcpStream,
    channel: Receiver<Command>,
}

impl Thread {
    pub fn new(socket: TcpStream, channel: Receiver<Command>) -> Thread {
        Thread { socket, channel }
    }

    pub fn run(&mut self) {
        loop {
            let cmd = self.channel.recv().unwrap().to_network();
            match bincode::serialize(&cmd) {
                Err(e) => {
                    eprintln!(
                        "An error has occurred while encoding network command: {}",
                        e
                    );
                }
                Ok(mut v) => {
                    let mut buf: [u8; 4] = [0; 4];
                    LittleEndian::write_u32(&mut buf, v.len() as u32);
                    v.insert(0, buf[3]);
                    v.insert(0, buf[2]);
                    v.insert(0, buf[1]);
                    v.insert(0, buf[0]);
                    if let Err(e) = self.socket.write_all(&v) {
                        eprintln!("An error has occurred while sending network command: {}", e);
                    }
                }
            };
            if cmd == NetCommand::Terminate {
                break;
            }
        }
    }
}
