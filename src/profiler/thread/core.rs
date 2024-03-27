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

use crate::profiler::log_msg::{EventLog, SpanLog};
use crate::profiler::network_types as nt;
use crate::profiler::network_types::{Hello, MatchResult, HELLO_PACKET};
use crate::profiler::state::ChannelsOut;
use crate::profiler::thread::{command, FixedBufStr};
use crate::profiler::thread::util::read_command_line;
use bp3d_os::cpu_info::read_cpu_info;
use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::sync::oneshot;
use crate::profiler::thread::util::wrap_io_debug_error;

use super::net::Net;
use super::store::SpanStore;

struct Thread<'a> {
    channels: ChannelsOut,
    msg: [u8; 1024],
    net: Net<'a>,
    core: SpanStore,
}

impl<'a> Thread<'a> {
    pub fn new(
        socket: &'a mut TcpStream,
        channels: ChannelsOut,
        config: nt::message::ClientConfig,
        max_rows: u32,
        min_period: u16,
    ) -> Thread {
        Thread {
            channels,
            msg: [0; 1024],
            net: Net::new(socket),
            core: SpanStore::new(max_rows, min_period, &config),
        }
    }

    async fn handle_span_data(&mut self, log: SpanLog) {
        if let Some(msg) = self.core.record(log) {
            wrap_io_debug_error!(self.net.network_write_fixed(msg).await);
            wrap_io_debug_error!(self.net.flush().await);
        }
    }

    async fn handle_span(&mut self, cmd: command::Span) {
        match cmd {
            command::Span::Log(msg) => self.handle_span_data(msg).await,
            command::Span::Event(msg) => self.handle_event(msg).await,
            command::Span::Alloc { id, metadata } => {
                self.core.reserve_span(id);
                let msg = nt::message::SpanAlloc {
                    id: id.get(),
                    metadata: nt::message::Metadata {
                        level: nt::message::Level::from_tracing(*metadata.level()),
                        file: metadata.file(),
                        line: metadata.line(),
                        module_path: metadata.module_path(),
                        name: metadata.name(),
                        target: metadata.target(),
                    },
                };
                wrap_io_debug_error!(self.net.network_write_dyn(msg, &mut self.msg).await);
            }
            command::Span::UpdateParent { id, parent } => {
                let msg = nt::message::SpanParent {
                    id: id.get(),
                    parent_node: parent.map(|v| v.get()).unwrap_or(0),
                };
                wrap_io_debug_error!(self.net.network_write_fixed(msg).await);
            }
            command::Span::Follows { id, follows } => {
                let id = id.get_id();
                let follows = follows.get_id();
                let msg = nt::message::SpanFollows {
                    id: id.get(),
                    follows: follows.get(),
                };
                wrap_io_debug_error!(self.net.network_write_fixed(msg).await);
            }
        }
    }

    async fn handle_event(&mut self, mut event: EventLog) {
        event.write_finish();
        let msg = nt::message::SpanEvent {
            id: event.id().map(|v| v.get()).unwrap_or(0),
            level: event.level(),
            timestamp: event.timestamp(),
        };
        wrap_io_debug_error!(self.net.network_write_fixed_payload(msg, event.as_bytes()).await);
    }

    async fn handle_control(&mut self, command: command::Control) -> bool {
        match command {
            command::Control::Project {
                app_name,
                name,
                version,
            } => {
                let mut cmd_line: FixedBufStr<255> = FixedBufStr::new();
                read_command_line(&mut cmd_line);
                let app_name = app_name.str();
                let name = name.str();
                let version = version.str();
                let info = read_cpu_info();
                let msg = nt::message::Project {
                    app_name,
                    name,
                    version,
                    target: nt::message::Target {
                        arch: std::env::consts::ARCH,
                        family: std::env::consts::FAMILY,
                        os: std::env::consts::OS,
                    },
                    cpu: info.map(|v| nt::message::Cpu {
                        name: v.name,
                        core_count: v.core_count,
                    }),
                    cmd_line: cmd_line.str()
                };
                wrap_io_debug_error!(self.net.network_write_dyn(msg, &mut self.msg).await);
                true
            }
            command::Control::Terminate => {
                wrap_io_debug_error!(self.net.flush().await);
                self.core.stop_recording(&mut self.net).await;
                wrap_io_debug_error!(self.net.flush().await);
                false
            }
        }
    }

    async fn handle_net_command(&mut self, command: std::io::Result<nt::message::ClientRecord>) {
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
                cmd = self.net.network_read_fixed::<nt::message::ClientRecord>() => self.handle_net_command(cmd).await,
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

async fn init(
    port: u16,
    max_rows: u32,
    min_period: u16,
) -> std::io::Result<(TcpStream, nt::message::ClientConfig)> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let listener = TcpListener::bind(addr).await?;
    let (mut socket, _) = listener.accept().await?;
    handle_hello(&mut socket).await?;
    let mut net = Net::new(&mut socket);
    let msg = nt::message::ServerConfig {
        max_rows,
        min_period,
    };
    net.network_write_fixed(msg).await?;
    net.flush().await?;
    let config: nt::message::ClientConfig = net.network_read_fixed().await?;
    Ok((socket, config))
}

pub fn run(
    port: u16,
    mut channels: ChannelsOut,
    max_rows: u32,
    min_period: u16,
    result_channel: oneshot::Sender<std::io::Result<Option<nt::message::Level>>>,
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
            res = init(port, max_rows, min_period) => {
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
