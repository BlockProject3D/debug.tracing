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

use tokio::sync::oneshot;
use std::collections::HashMap;
use crate::profiler::cpu_info::read_cpu_info;
use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::num::NonZeroU32;
use std::path::PathBuf;
use serde::Serialize;
use tokio::fs::File;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use crate::profiler::network_types as nt;
use crate::profiler::state::ChannelsOut;
use crate::profiler::thread::command;
use crate::profiler::thread::state::SpanData;
use crate::profiler::thread::util::read_command_line;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use crate::profiler::log_msg::{EventLog, SpanLog};
use crate::profiler::network_types::{Hello, HELLO_PACKET, MatchResult};

struct Net {
    socket: BufWriter<TcpStream>,
    head_buffer: [u8; 64],
    net_buffer: [u8; 1024]
}

impl Net {
    pub fn new(socket: TcpStream) -> Net {
        Net {
            socket: BufWriter::new(socket),
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
    net: Net,
    logs: Option<PathBuf>
}

impl Thread {
    pub fn new(socket: TcpStream, channels: ChannelsOut, logs: Option<PathBuf>) -> Thread {
        Thread {
            channels,
            span_data: HashMap::new(),
            net: Net::new(socket),
            logs
        }
    }

    async fn create_runs_file(&self, id: NonZeroU32) -> Option<BufWriter<File>> {
        let filename = format!("{}.csv", id);
        File::create(self.logs.as_ref()?.join(filename)).await.ok().map(BufWriter::new)
    }

    async fn handle_span_data(&mut self, mut log: SpanLog) {
        if let Some(data) = self.span_data.get_mut(&log.id()) {
            let duration = log.get_duration();
            if data.update(&duration) {
                let head = nt::header::SpanUpdate {
                    id: log.id().get(),
                    run_count: data.run_count,
                    average_time: nt::header::Duration::from(&data.get_average()),
                    min_time: nt::header::Duration::from(&data.min_time),
                    max_time: nt::header::Duration::from(&data.max_time)
                };
                self.net.network_write(head).await;
            }
            if !data.has_overflowed { //Hard limit on the number of rows in the CSV to
                // avoid disk overload.
                if let Some(file) = &mut data.runs_file {
                    use std::fmt::Write;
                    let _ = write!(log, ",{},{},{}",
                                   duration.as_secs(), duration.subsec_millis(),
                                   duration.subsec_micros() - (duration.subsec_millis() * 1000));
                    let _ = file.write_all(log.msg().as_bytes()).await;
                    let _ = file.write_all("\n".as_bytes()).await;
                }
            }
        }
    }

    async fn handle_span(&mut self, cmd: command::Span) {
        match cmd {
            command::Span::Log(msg) => self.handle_span_data(msg).await,
            command::Span::Alloc { id, metadata } => {
                self.span_data.insert(id, SpanData::new(self.create_runs_file(id).await));
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
            command::Span::UpdateParent { id, parent } => {
                self.net.network_write(nt::header::SpanParent {
                    id: id.get(),
                    parent_node: parent.map(|v| v.get()).unwrap_or(0)
                }).await;
            },
            command::Span::Follows { id, follows } => {
                let id = id.get_id();
                let follows = follows.get_id();
                let head = nt::header::SpanFollows {
                    id: id.get(),
                    follows: follows.get()
                };
                self.net.network_write(head).await;
            },
        }
    }

    async fn handle_event(&mut self, event: EventLog) {
        let mut payload = self.net.get_payload();
        let head = nt::header::SpanEvent {
            id: event.id().map(|v| v.get()).unwrap_or(0),
            message: payload.write_object(event.msg()).unwrap(),
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
                cmd = self.channels.span.recv() => if let Some(cmd) = cmd { self.handle_span(cmd).await },
                cmd = self.channels.event.recv() => if let Some(cmd) = cmd { self.handle_event(cmd).await },
                cmd = self.channels.control.recv() => if let Some(cmd) = cmd {
                    if !self.handle_control(cmd).await {
                        break
                    }
                }
            }
        }
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
    //TODO: Re-arm when preliminary performance tests are over
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
