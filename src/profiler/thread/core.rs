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

use std::collections::HashMap;
use crate::profiler::cpu_info::read_cpu_info;
use crate::util::span_to_id_instance;
use crossbeam_channel::Receiver;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use serde::Serialize;
use tracing_core::span::Id;
use crate::profiler::network_types as nt;
use crate::profiler::thread::Command;
use crate::profiler::thread::state::{SpanData, SpanInstance};
use crate::profiler::thread::util::read_command_line;

const OVERFLOW_LIMIT: u32 = 1_000_000_000;

struct Net {
    socket: TcpStream,
    head_buffer: [u8; 64],
    net_buffer: [u8; 1024]
}

impl Net {
    pub fn new(socket: TcpStream) -> Net {
        Net {
            socket,
            head_buffer: [0; 64],
            net_buffer: [0; 1024]
        }
    }

    pub fn get_payload(&mut self) -> nt::util::Payload {
        nt::util::Payload::new(&mut self.net_buffer)
    }

    pub fn network_write<H: Serialize + nt::header::MsgHeader>(&mut self, header: H) {
        let head_len = {
            let mut serializer = nt::serializer::Serializer::new(&mut self.head_buffer);
            if let Err(_) = H::TYPE.serialize(&mut serializer) {
                return;
            }
            if let Err(_) = header.serialize(&mut serializer) {
                return;
            }
            serializer.length()
        };
        if let Err(e) = self.socket.write(&self.head_buffer[..head_len]) {
            eprintln!("Failed to write to network: {}", e);
            return;
        }
        if H::HAS_PAYLOAD {
            //Write the entire network buffer no matter what because Rust borrow checker refuses to
            // allow passing the size of the payload...
            if let Err(e) = self.socket.write(&self.net_buffer) {
                eprintln!("Failed to write to network: {}", e);
            }
        }
    }
}

pub struct Thread {
    channel: Receiver<Command>,
    span_data: HashMap<u32, SpanData>,
    span_instances: HashMap<u64, SpanInstance>,
    net: Net,
    logs: Option<PathBuf>
}

impl Thread {
    pub fn new(socket: TcpStream, channel: Receiver<Command>, logs: Option<PathBuf>) -> Thread {
        Thread {
            channel,
            span_data: HashMap::new(),
            span_instances: HashMap::new(),
            net: Net::new(socket),
            logs
        }
    }

    fn create_runs_file(&self, id: u32) -> Option<BufWriter<File>> {
        let filename = format!("{}.csv", id);
        File::create(self.logs.as_ref()?.join(filename)).ok().map(BufWriter::new)
    }

    pub fn run(&mut self) {
        loop {
            let cmd = self.channel.recv().unwrap();
            match cmd {
                Command::Project { app_name, name, version } => {
                    let mut payload = self.net.get_payload();
                    let app_name = app_name.str();
                    let name = name.str();
                    let version = version.str();
                    let info = read_cpu_info();
                    let head = nt::header::Project {
                        app_name: payload.write_object(app_name).unwrap(),
                        name: payload.write_object(name).unwrap(),
                        version: payload.write_object(version).unwrap(),
                        target: nt::header::Target {
                            arch: payload.write_object(std::env::consts::ARCH).unwrap(),
                            family: payload.write_object(std::env::consts::FAMILY).unwrap(),
                            os: payload.write_object(std::env::consts::OS).unwrap()
                        },
                        cpu: info.map(|v| nt::header::Cpu {
                            name: payload.write_object(&*v.name).unwrap(),
                            core_count: v.core_count
                        }),
                        cmd_line: read_command_line(&mut payload)
                    };
                    self.net.network_write(head);
                },
                Command::SpanAlloc { id, metadata } => {
                    let (id, _) = span_to_id_instance(&Id::from_u64(id));
                    self.span_data.insert(id, SpanData::new(metadata.name(), self.create_runs_file(id)));
                    let mut payload = self.net.get_payload();
                    let head = nt::header::SpanAlloc {
                        id,
                        metadata: nt::header::Metadata {
                            level: nt::header::Level::from_tracing(*metadata.level()),
                            file: metadata.file().map(|v| payload.write_object(v).unwrap()),
                            line: metadata.line(),
                            module_path: metadata.module_path().map(|v| payload.write_object(v).unwrap()),
                            name: payload.write_object(metadata.name()).unwrap(),
                            target: payload.write_object(metadata.target()).unwrap()
                        }
                    };
                    self.net.network_write(head);
                },
                Command::SpanInit { span, parent/*, message, value_set, payload*/ } => {
                    let (id, instance_id) = span_to_id_instance(&Id::from_u64(span));
                    let parent = parent.map(|v| v as _);
                    if let Some(data) = self.span_data.get_mut(&id) {
                        if data.parent != parent {
                            self.net.network_write(nt::header::SpanParent {
                                id,
                                parent
                            });
                            data.parent = parent;
                        }
                        data.instance_count += 1;
                        let instance = self.span_instances.entry(span).or_insert_with(SpanInstance::new);
                        let _ = write!(instance.csv_row, "{},", instance_id);
                    }
                },
                Command::SpanValue { span, key, value } => {
                    if let Some(instance) = self.span_instances.get_mut(&span) {
                        instance.append_value(key, &value)
                    }
                },
                Command::SpanMessage { span, message } => {
                    if let Some(instance) = self.span_instances.get_mut(&span) {
                        instance.append_message(&message)
                    }
                },
                Command::SpanFollows { span, follows } => {
                    let (id, _) = span_to_id_instance(&Id::from_u64(span));
                    let (follows, _) = span_to_id_instance(&Id::from_u64(follows));
                    let head = nt::header::SpanFollows {
                        id,
                        follows
                    };
                    self.net.network_write(head);
                },
                Command::Event { id, timestamp, message } => {
                    let mut payload = self.net.get_payload();
                    let head = nt::header::SpanEvent {
                        id,
                        message: payload.write_object(message.msg()).unwrap(),
                        target: payload.write_object(message.target()).unwrap(),
                        level: nt::header::Level::from_log(message.level()),
                        timestamp
                    };
                    self.net.network_write(head);
                },
                Command::SpanEnter(_) => {},
                Command::SpanExit { span, duration } => {
                    let (id, _) = span_to_id_instance(&Id::from_u64(span));
                    if let Some(data) = self.span_data.get_mut(&id) {
                        data.run_count += 1;
                        //Avoid overflow.
                        if data.run_count > OVERFLOW_LIMIT {
                            data.total_time = Duration::ZERO;
                            data.run_count = 0;
                            data.has_overflowed = true;
                        }
                        if !data.has_overflowed { //Hard limit on the number of rows in the CSV to
                            // avoid disk overload.
                            if let Some(mut instance) = self.span_instances.remove(&span) {
                                instance.finish(&duration, data.name);
                                if let Some(file) = &mut data.runs_file {
                                    instance.write(file);
                                }
                            }
                        }
                        if duration > data.max_time {
                            data.max_time = duration;
                        }
                        if duration < data.min_time {
                            data.min_time = duration;
                        }
                        data.total_time += duration;
                    }
                },
                Command::SpanFree(span) => {
                    let (id, _) = span_to_id_instance(&Id::from_u64(span));
                    if let Some(data) = self.span_data.get_mut(&id) {
                        data.instance_count -= 1;
                    }
                }
                Command::Terminate => {
                    for (_, v) in &mut self.span_data {
                        let _ = v.runs_file.take();
                    }
                    break;
                }
            }
        }
    }
}
