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

use tokio::sync::mpsc;
use tokio::sync::oneshot;
use std::collections::HashMap;
use crate::profiler::cpu_info::read_cpu_info;
use crate::util::{span_to_id_instance, SpanId};
use crossbeam_channel::Receiver;
use std::io::{Error, ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;
use serde::Serialize;
use tokio::fs::{File, OpenOptions};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tracing_core::span::Id;
use crate::profiler::network_types as nt;
use crate::profiler::state::ChannelsOut;
use crate::profiler::thread::Command;
use crate::profiler::thread::command;
use crate::profiler::thread::state::{SpanData, SpanInstance};
use crate::profiler::thread::util::read_command_line;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use crate::profiler::log_msg::{EventLog, SpanLog};
use crate::profiler::network_types::{Hello, HELLO_PACKET, MatchResult};

const OVERFLOW_LIMIT: u32 = 1_000_000_000;

struct Net {
    socket: BufReader<TcpStream>,
    head_buffer: [u8; 64],
    net_buffer: [u8; 1024]
}

impl Net {
    pub fn new(socket: TcpStream) -> Net {
        Net {
            socket: BufReader::new(socket),
            head_buffer: [0; 64],
            net_buffer: [0; 1024]
        }
    }

    pub fn get_payload(&mut self) -> nt::util::Payload {
        nt::util::Payload::new(&mut self.net_buffer)
    }

    pub async fn network_write<H: Serialize + nt::header::MsgHeader>(&mut self, header: H) {
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
        if let Err(e) = self.socket.write_all(&self.head_buffer[..head_len]).await {
            eprintln!("Failed to write to network: {}", e);
            return;
        }
        if H::HAS_PAYLOAD {
            //Write the entire network buffer no matter what because Rust borrow checker refuses to
            // allow passing the size of the payload...
            if let Err(e) = self.socket.write_all(&self.net_buffer).await {
                eprintln!("Failed to write to network: {}", e);
            }
        }
    }
}

struct Thread {
    channels: ChannelsOut,
    span_data: HashMap<NonZeroU32, SpanData>,
    //span_instances: HashMap<SpanId, SpanInstance>,
    net: Net,
    logs: Option<PathBuf>
}

impl Thread {
    pub fn new(socket: TcpStream, channels: ChannelsOut, logs: Option<PathBuf>) -> Thread {
        Thread {
            channels,
            span_data: HashMap::new(),
            //span_instances: HashMap::new(),
            net: Net::new(socket),
            logs
        }
    }

    async fn create_runs_file(&self, id: NonZeroU32) -> Option<BufWriter<File>> {
        let filename = format!("{}.csv", id);
        File::create(self.logs.as_ref()?.join(filename)).await.ok().map(BufWriter::new)
    }

    async fn handle_span_data(&mut self, mut log: SpanLog/*, span: command::Span<command::SpanData>*/) {
        /*match span.ty {
            command::SpanData::Value { key, value } => {
                if let Some(instance) = self.span_instances.get_mut(&span.id) {
                    instance.append_value(key, &value)
                }
            },
            command::SpanData::Message { message } => {
                if let Some(instance) = self.span_instances.get_mut(&span.id) {
                    instance.append_message(&message)
                }
            }
        }*/
        if let Some(data) = self.span_data.get_mut(&log.id()) {
            let duration = log.get_duration();
            data.run_count += 1;
            //Avoid overflow.
            if data.run_count > OVERFLOW_LIMIT {
                data.total_time = Duration::ZERO;
                data.run_count = 0;
                data.has_overflowed = true;
            }
            if !data.has_overflowed { //Hard limit on the number of rows in the CSV to
                // avoid disk overload.
                /*if let Some(instance) = self.span_instances.get_mut(&span.id) {
                    instance.finish(&duration, data.name);
                    if let Some(file) = &mut data.runs_file {
                        instance.write(file).await;
                    }
                    instance.reset();
                }*/
                if let Some(file) = &mut data.runs_file {
                    use std::fmt::Write;
                    let _ = write!(log, ",{},{},{}",
                                   duration.as_secs(), duration.subsec_millis(),
                                   duration.subsec_micros() - (duration.subsec_millis() * 1000));
                    let _ = file.write_all(log.msg().as_bytes()).await;
                    let _ = file.write_all("\n".as_bytes()).await;
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
    }

    async fn handle_span_control(&mut self, span: command::Span<command::SpanControl>) {
        match span.ty {
            /*command::SpanControl::Value { key, value } => {
                if let Some(instance) = self.span_instances.get_mut(&span.id) {
                    instance.append_value(key, &value)
                }
            },
            command::SpanControl::Message { message } => {
                if let Some(instance) = self.span_instances.get_mut(&span.id) {
                    instance.append_message(&message)
                }
            },*/
            command::SpanControl::Alloc { metadata } => {
                let id = span.id.get_id();
                self.span_data.insert(id, SpanData::new(metadata.name(), self.create_runs_file(id).await));
                let mut payload = self.net.get_payload();
                let head = nt::header::SpanAlloc {
                    id: id.get(),
                    metadata: nt::header::Metadata {
                        level: nt::header::Level::from_tracing(*metadata.level()),
                        file: metadata.file().map(|v| payload.write_object(v).unwrap()),
                        line: metadata.line(),
                        module_path: metadata.module_path().map(|v| payload.write_object(v).unwrap()),
                        name: payload.write_object(metadata.name()).unwrap(),
                        target: payload.write_object(metadata.target()).unwrap()
                    }
                };
                self.net.network_write(head).await;
            },
            command::SpanControl::UpdateParent { parent } => {
                self.net.network_write(nt::header::SpanParent {
                    id: span.id.get_id().get(),
                    parent_node: parent.map(|v| v.get()).unwrap_or(0)
                }).await;
            },
            /*command::SpanControl::Init { parent } => {
                let id = span.id.get_id();
                let parent_node = parent.map(|v| v.get_id());
                if let Some(data) = self.span_data.get_mut(&id) {
                    if data.parent != parent_node {
                        self.net.network_write(nt::header::SpanParent {
                            id: id.get(),
                            parent_node: parent_node.map(|v| v.get()).unwrap_or(0)
                        }).await;
                        data.parent = parent_node;
                    }
                    data.instance_count += 1;
                    let instance = self.span_instances.entry(span.id).or_insert_with(SpanInstance::new);
                    let _ = write!(instance.csv_row, "{},", parent.map(|v| v.into_u64()).unwrap_or(0));
                }
            },*/
            command::SpanControl::Follows { follows } => {
                let id = span.id.get_id();
                let follows = follows.get_id();
                let head = nt::header::SpanFollows {
                    id: id.get(),
                    follows: follows.get()
                };
                self.net.network_write(head).await;
            },
            /*command::SpanControl::Exit { duration } => {
                let id = span.id.get_id();
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
                        if let Some(instance) = self.span_instances.get_mut(&span.id) {
                            instance.finish(&duration, data.name);
                            if let Some(file) = &mut data.runs_file {
                                instance.write(file).await;
                            }
                            instance.reset();
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
            },*/
            /*command::SpanControl::Free => {
                let id = span.id.get_id();
                if let Some(data) = self.span_data.get_mut(&id) {
                    data.instance_count -= 1;
                }
            }*/
        }
    }

    async fn handle_event(&mut self, event: EventLog) {
        let mut payload = self.net.get_payload();
        let head = nt::header::SpanEvent {
            id: event.id().map(|v| v.get()).unwrap_or(0),
            message: payload.write_object(event.msg()).unwrap(),
            //target: payload.write_object(event.target()).unwrap(),
            level: event.level(),
            timestamp: event.timestamp()
        };
        self.net.network_write(head).await;
    }

    async fn handle_control(&mut self, command: command::Control) -> bool {
        match command {
            command::Control::Project { app_name, name, version } => {
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
                self.net.network_write(head).await;
                true
            },
            command::Control::Terminate => {
                for (_, v) in &mut self.span_data {
                    if let Some(mut v) = v.runs_file.take() {
                        let _ = v.flush().await;
                    }
                }
                false
            }
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                cmd = self.channels.span.recv() => if let Some(cmd) = cmd { self.handle_span_data(cmd).await },
                cmd = self.channels.span_control.recv() => if let Some(cmd) = cmd { self.handle_span_control(cmd).await },
                //cmd = self.channels.span_data.recv() => if let Some(cmd) = cmd { self.handle_span_data(cmd).await },
                cmd = self.channels.event.recv() => if let Some(cmd) = cmd { self.handle_event(cmd).await },
                cmd = self.channels.control.recv() => if let Some(cmd) = cmd {
                    if !self.handle_control(cmd).await {
                        break
                    }
                }
            }
        }
        /*loop {
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
        }*/
    }
}

async fn handle_hello(client: &mut TcpStream) -> std::io::Result<()> {
    let bytes = HELLO_PACKET.to_bytes();
    let mut block = [0; 40];
    client.write(&bytes).await?;
    client.read_exact(&mut block).await?;
    let packet = Hello::from_bytes(block);
    match HELLO_PACKET.matches(&packet) {
        MatchResult::SignatureMismatch => {
            Err(Error::new(ErrorKind::Other, "protocol signature mismatch"))
        }
        MatchResult::VersionMismatch => {
            Err(Error::new(ErrorKind::Other, "version signature mismatch"))
        }
        MatchResult::Ok => Ok(()),
    }
}

async fn init(port: u16) -> std::io::Result<TcpStream> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let listener = TcpListener::bind(addr).await?;
    let (mut socket, _) = listener.accept().await?;
    //handle_hello(&mut socket).await?;
    Ok(socket)
}

pub fn run(port: u16, mut channels: ChannelsOut, logs: Option<PathBuf>, result_channel: oneshot::Sender<std::io::Result<()>>) {
    Builder::new_current_thread().enable_io().build().unwrap().block_on(async {
        tokio::select! {
            cmd = channels.control.recv() => {
                match cmd.unwrap() {
                    command::Control::Terminate => {
                        return;
                    },
                    _ => ()
                }
            },
            res = init(port) => {
                let socket = match res {
                    Ok(v) => {
                        result_channel.send(Ok(())).unwrap();
                        v
                    },
                    Err(e) => {
                        result_channel.send(Err(e)).unwrap();
                        return
                    }
                };
                let mut thread = Thread::new(socket, channels, logs);
                thread.run().await;
            }
        }
    });
}
