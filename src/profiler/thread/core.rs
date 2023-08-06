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

use crate::profiler::cpu_info::read_cpu_info;
use crate::profiler::log_msg::{EventLog, SpanLog};
use crate::profiler::network_types as nt;
use crate::profiler::network_types::{Hello, MatchResult, HELLO_PACKET};
use crate::profiler::state::ChannelsOut;
use crate::profiler::thread::command;
use crate::profiler::thread::state::SpanData;
use crate::profiler::thread::util::read_command_line;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::num::NonZeroU32;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::sync::oneshot;

use super::store::SpanStore;

pub struct Net<'a> {
    write: BufWriter<WriteHalf<'a>>,
    head_buffer: [u8; 64],
    cursor: usize,
    net_buffer: [u8; 1024],
    read: BufReader<ReadHalf<'a>>,
}

impl<'a> Net<'a> {
    pub fn new(socket: &'a mut TcpStream) -> Net<'a> {
        let (read, write) = socket.split();
        Net {
            write: BufWriter::new(write),
            read: BufReader::new(read),
            head_buffer: [0; 64],
            cursor: 0,
            net_buffer: [0; 1024],
        }
    }

    pub async fn network_read<'b, M: nt::header::MsgSize + Deserialize<'b>>(
        &'b mut self,
    ) -> std::io::Result<M> {
        self.read
            .read_exact(&mut self.head_buffer[0..M::SIZE])
            .await?;
        let mut de = nt::deserializer::Deserializer::new(&self.head_buffer[0..M::SIZE]);
        M::deserialize(&mut de).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn get_payload(&mut self) -> nt::util::Payload {
        self.cursor = 0;
        nt::util::Payload::new(&mut self.net_buffer, &mut self.cursor)
    }

    pub async fn network_write_raw<H: Serialize + nt::header::MsgHeader>(&mut self, header: H) -> std::io::Result<()> {
        let mut head_len = 0;
        let mut serializer = nt::serializer::Serializer::new(&mut self.head_buffer, &mut head_len);
        if let Err(e) = H::TYPE.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        if let Err(e) = header.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        self.write.write_all(&self.head_buffer[..head_len]).await?;
        if H::HAS_PAYLOAD {
            self.write.write_all(&self.net_buffer[..self.cursor]).await?;
        }
        Ok(())
    }

    pub async fn network_write<H: Serialize + nt::header::MsgHeader>(&mut self, header: H) {
        if let Err(e) = self.network_write_raw(header).await {
            eprintln!("Failed to write to network: {}", e);
        }
    }
}

struct Thread<'a> {
    channels: ChannelsOut,
    span_data: HashMap<NonZeroU32, SpanData>,
    net: Net<'a>,
    core: SpanStore,
}

impl<'a> Thread<'a> {
    pub fn new(
        socket: &'a mut TcpStream,
        channels: ChannelsOut,
        config: nt::header::ClientConfig,
        max_rows: u32,
        min_period: u16,
    ) -> Thread {
        Thread {
            channels,
            span_data: HashMap::new(),
            net: Net::new(socket),
            core: SpanStore::new(max_rows, min_period, &config),
        }
    }

    async fn handle_span_data(&mut self, log: SpanLog) {
        if let Some(msg) = self.core.record(log) {
            self.net.network_write(msg).await;
        }
    }

    async fn handle_span(&mut self, cmd: command::Span) {
        match cmd {
            command::Span::Log(msg) => self.handle_span_data(msg).await,
            command::Span::Event(msg) => self.handle_event(msg).await,
            command::Span::Alloc { id, metadata } => {
                self.span_data.insert(id, SpanData::new());
                let mut payload = self.net.get_payload();
                let head = nt::header::SpanAlloc {
                    id: id.get(),
                    metadata: nt::header::Metadata {
                        level: nt::header::Level::from_tracing(*metadata.level()),
                        file: metadata.file().map(|v| payload.write_object(v).unwrap()),
                        line: metadata.line(),
                        module_path: metadata
                            .module_path()
                            .map(|v| payload.write_object(v).unwrap()),
                        name: payload.write_object(metadata.name()).unwrap(),
                        target: payload.write_object(metadata.target()).unwrap(),
                    },
                };
                self.net.network_write(head).await;
            }
            command::Span::UpdateParent { id, parent } => {
                self.net
                    .network_write(nt::header::SpanParent {
                        id: id.get(),
                        parent_node: parent.map(|v| v.get()).unwrap_or(0),
                    })
                    .await;
            }
            command::Span::Follows { id, follows } => {
                let id = id.get_id();
                let follows = follows.get_id();
                let head = nt::header::SpanFollows {
                    id: id.get(),
                    follows: follows.get(),
                };
                self.net.network_write(head).await;
            }
        }
    }

    async fn handle_event(&mut self, event: EventLog) {
        let mut payload = self.net.get_payload();
        let head = nt::header::SpanEvent {
            id: event.id().map(|v| v.get()).unwrap_or(0),
            message: payload.write_object(event.msg()).unwrap(),
            level: event.level(),
            timestamp: event.timestamp(),
        };
        self.net.network_write(head).await;
    }

    async fn handle_control(&mut self, command: command::Control) -> bool {
        match command {
            command::Control::Project {
                app_name,
                name,
                version,
            } => {
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
                        os: payload.write_object(std::env::consts::OS).unwrap(),
                    },
                    cpu: info.map(|v| nt::header::Cpu {
                        name: payload.write_object(&*v.name).unwrap(),
                        core_count: v.core_count,
                    }),
                    cmd_line: read_command_line(&mut payload),
                };
                self.net.network_write(head).await;
                true
            }
            command::Control::Terminate => {
                let _ = self.net.write.flush().await;
                self.core.stop_recording(&mut self.net).await;
                let _ = self.net.write.flush().await;
                false
            }
        }
    }

    async fn handle_net_command(&mut self, command: std::io::Result<nt::header::ClientRecord>) {
        match command {
            Ok(v) => {
                if v.enable {
                    self.core.start_recording(v.max_rows);
                } else {
                    self.core.stop_recording(&mut self.net).await;
                }
            }
            Err(e) => println!("Failed to read network command: {}", e),
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                cmd = self.net.network_read::<nt::header::ClientRecord>() => self.handle_net_command(cmd).await,
                cmd = self.channels.span.recv() => if let Some(cmd) = cmd { self.handle_span(cmd).await },
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
        MatchResult::VersionMismatch => Err(Error::new(ErrorKind::Other, "version mismatch")),
        MatchResult::Ok => Ok(()),
    }
}

async fn init(port: u16, max_rows: u32) -> std::io::Result<(TcpStream, nt::header::ClientConfig)> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let listener = TcpListener::bind(addr).await?;
    let (mut socket, _) = listener.accept().await?;
    handle_hello(&mut socket).await?;
    let mut net = Net::new(&mut socket);
    let head = nt::header::ServerConfig { max_rows };
    net.network_write_raw(head).await?;
    net.write.flush().await?;
    let config: nt::header::ClientConfig = net.network_read().await?;
    Ok((socket, config))
}

pub fn run(
    port: u16,
    mut channels: ChannelsOut,
    max_rows: u32,
    min_period: u16,
    result_channel: oneshot::Sender<std::io::Result<Option<nt::header::Level>>>,
) {
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
            res = init(port, max_rows) => {
                let (mut socket, config) = match res {
                    Ok((socket, config)) => {
                        result_channel.send(Ok(config.max_level)).unwrap();
                        (socket, config)
                    },
                    Err(e) => {
                        result_channel.send(Err(e)).unwrap();
                        return
                    }
                };
                let mut thread = Thread::new(&mut socket, channels, config, max_rows, min_period);
                thread.run().await;
            }
        }
    });
}
