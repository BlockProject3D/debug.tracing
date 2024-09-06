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

use crate::profiler::log_msg::{EventLog, ProfilerRecord};
use crate::profiler::network as net;
use crate::profiler::thread::util::read_command_line;
use crate::profiler::thread::util::wrap_io_debug_error;
use bp3d_os::cpu_info::read_cpu_info;
use std::net::{Ipv4Addr, SocketAddrV4};
use bp3d_debug::profiler::section::Level;
use bp3d_proto::message::payload::List;
use bp3d_util::format::FixedBufStr;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::sync::oneshot;
use crate::profiler::thread::channels::ChannelsOut;
use crate::profiler::thread::command::{Control, Execution};

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
        config: net::client::Config<[u8; net::client::SIZE_CONFIG]>,
        max_rows: u32,
        min_period: u16,
    ) -> Thread {
        Thread {
            channels,
            msg: [0; 1024],
            net: Net::new(socket),
            core: SpanStore::new(max_rows, min_period, &config.to_ref()),
        }
    }

    async fn handle_span_data(&mut self, log: ProfilerRecord) {
        if let Some(msg) = self.core.record(log) {
            wrap_io_debug_error!(self.net.network_write_fixed(net::message::Type::ProfilerSectionUpdate, msg).await);
            wrap_io_debug_error!(self.net.flush().await);
        }
    }

    async fn handle_span(&mut self, cmd: Execution) {
        match cmd {
            Execution::ProfilerRecord(msg) => self.handle_span_data(msg).await,
            Execution::Event(msg) => self.handle_event(msg).await,
            Execution::SpanEnter { start, fields } => {
                let msg = net::span::Enter {
                    id: fields.id(),
                    start,
                    fields: List::from_raw_parts(fields.as_bytes(), fields.var_count() as _)
                };
                wrap_io_debug_error!(self.net.network_write_dyn_payload(net::message::Type::SpanEnter, msg).await);
            }
            Execution::SpanRecord(fields) => {
                let msg = net::span::Record {
                    id: fields.id(),
                    fields: List::from_raw_parts(fields.as_bytes(), fields.var_count() as _)
                };
                wrap_io_debug_error!(self.net.network_write_dyn_payload(net::message::Type::SpanRecord, msg).await);
            }
            Execution::SpanExit { id, end } => {
                let mut msg = net::span::Exit::new_on_stack();
                msg.set_end(end).get_id_mut().set_instance(id.get_instance().get()).set_callsite(id.get_callsite().get());
                wrap_io_debug_error!(self.net.network_write_fixed(net::message::Type::SpanExit, msg).await);
            }
            /*command::Span::Alloc { id, metadata } => {
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
            }*/
        }
    }

    async fn handle_event(&mut self, event: EventLog) {
        let msg = net::event::Log {
            header: event.header(),
            location: net::common::Location {
                module_path: event.location().module_path(),
                file: event.location().file(),
                line: event.location().line()
            },
            fields: List::from_raw_parts(event.as_bytes(), event.var_count() as _)
        };
        wrap_io_debug_error!(self.net.network_write_dyn_payload(net::message::Type::Event, msg).await);
    }

    async fn handle_control(&mut self, command: Control) -> bool {
        match command {
            Control::Project {
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
                let msg = net::message::Project {
                    app_name,
                    name,
                    version,
                    target: net::message::Target {
                        arch: std::env::consts::ARCH,
                        family: std::env::consts::FAMILY,
                        os: std::env::consts::OS,
                    },
                    cpu: info.as_ref().map(|v| net::message::Cpu {
                        name: &v.name,
                        core_count: v.core_count,
                    }),
                    cmd_line: cmd_line.str(),
                };
                wrap_io_debug_error!(self.net.network_write_dyn_payload(net::message::Type::Project, msg).await);
                true
            }
            Control::Terminate => {
                wrap_io_debug_error!(self.net.flush().await);
                self.core.stop_recording(&mut self.net).await;
                wrap_io_debug_error!(self.net.flush().await);
                false
            }
            Control::RegisterSection { section, id, parent } => {
                let mut header = net::profiler::SectionHeader::new_on_stack();
                header.set_id(id.get()).set_parent(parent.map(|v| v.get()).unwrap_or(0));
                match section.level() {
                    Level::Critical => header.set_level(net::profiler::Level::Critical),
                    Level::Periodic => header.set_level(net::profiler::Level::Periodic),
                    Level::Event => header.set_level(net::profiler::Level::Event)
                };
                let msg = net::profiler::Section {
                    header: header.to_ref(),
                    location: net::common::Location {
                        module_path: section.location().module_path(),
                        file: section.location().file(),
                        line: section.location().line()
                    },
                    name: section.name()
                };
                wrap_io_debug_error!(self.net.network_write_dyn_payload(net::message::Type::ProfilerSectionRegister, msg).await);
                true
            },
            Control::RegisterSpan { callsite, id } => {
                let msg = net::span::Callsite {
                    id: id.get(),
                    location: net::common::Location {
                        module_path: callsite.location().module_path(),
                        file: callsite.location().file(),
                        line: callsite.location().line()
                    },
                    name: callsite.name()
                };
                wrap_io_debug_error!(self.net.network_write_dyn_payload(net::message::Type::SpanCallsiteRegister, msg).await);
                true
            }
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                cmd = self.net.network_read_fixed::<net::client::Record<&[u8]>>() => {
                    match cmd {
                        Ok(v) => {
                            if v.get_enable() {
                                self.core.start_recording(v.get_max_rows());
                            } else {
                                self.core.stop_recording(&mut self.net).await;
                            }
                        },
                        Err(e) => println!("Failed to read network command: {}", e)
                    }
                },
                cmd = self.channels.execution.recv() => if let Some(cmd) = cmd { self.handle_span(cmd).await },
                cmd = self.channels.control.recv() => if let Some(cmd) = cmd {
                    if !self.handle_control(cmd).await {
                        break
                    }
                }
            }
        }
    }
}

/*async fn handle_hello(client: &mut TcpStream) -> std::io::Result<()> {
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
}*/

async fn init(
    port: u16,
    max_rows: u32,
    min_period: u16,
) -> std::io::Result<(TcpStream, net::client::Config<[u8; net::client::SIZE_CONFIG]>)> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let listener = TcpListener::bind(addr).await?;
    let (mut socket, _) = listener.accept().await?;
    //TODO: implement
    //handle_hello(&mut socket).await?;
    let mut net = Net::new(&mut socket);
    let mut msg = net::server::Config::new_on_stack();
    msg.set_max_rows(max_rows).set_min_period(min_period);
    net.network_write_fixed(net::message::Type::ServerConfig, msg).await?;
    net.flush().await?;
    let config: net::client::Config<&[u8]> = net.network_read_fixed().await?;
    let motherfuckingrust = config.copy_on_stack();
    Ok((socket, motherfuckingrust))
}

#[derive(Copy, Clone, Debug)]
pub struct Levels {
    pub section: Option<net::profiler::Level>,
    pub event: Option<net::event::Level>
}

pub fn run(
    port: u16,
    mut channels: ChannelsOut,
    max_rows: u32,
    min_period: u16,
    result_channel: oneshot::Sender<std::io::Result<Levels>>,
) {
    Builder::new_current_thread().enable_io().build().unwrap().block_on(async {
        tokio::select! {
            cmd = channels.control.recv() => {
                match cmd.unwrap() {
                    Control::Terminate => {
                        return;
                    },
                    _ => ()
                }
            },
            res = init(port, max_rows, min_period) => {
                let (mut socket, config) = match res {
                    Ok((socket, config)) => {
                        let levels = Levels {
                            section: config.get_max_profiler_level()
                                .map(|v| if v == net::profiler::Level::None { None } else { Some(v) })
                                .flatten(),
                            event: config.get_max_event_level()
                                .map(|v| if v == net::event::Level::None { None } else { Some(v) })
                                .flatten(),
                        };
                        result_channel.send(Ok(levels)).unwrap();
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
