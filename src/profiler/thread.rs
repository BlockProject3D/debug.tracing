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

use std::collections::HashMap;
use crate::profiler::cpu_info::read_cpu_info;
use crate::util::{Meta, span_to_id_instance};
use crossbeam_channel::Receiver;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::mem::MaybeUninit;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use bp3d_logger::LogMsg;
use serde::Serialize;
use tracing_core::span::Id;
use crate::profiler::network_types as nt;

#[derive(Clone, Debug)]
pub struct FixedBufStr<const N: usize> {
    buffer: [MaybeUninit<u8>; N],
    len: usize
}

impl<const N: usize> FixedBufStr<N> {
    pub fn new() -> FixedBufStr<N> {
        FixedBufStr {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0
        }
    }

    pub fn str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(std::mem::transmute(&self.buffer[..self.len]))
        }
    }

    pub fn from_str(value: &str) -> Self {
        let mut buffer = FixedBufStr::new();
        let len = std::cmp::min(value.len(), N);
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr(), std::mem::transmute(buffer.buffer.as_mut_ptr()), len);
        }
        buffer.len = len;
        buffer
    }

    pub fn from_debug<T: Debug>(value: T) -> Self {
        use std::fmt::Write;
        let mut buffer = FixedBufStr::new();
        let _ = write!(buffer, "{:?}", value);
        buffer
    }
}

impl<const N: usize> std::fmt::Write for FixedBufStr<N> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let len = std::cmp::min(value.len(), N);
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr(), std::mem::transmute(self.buffer.as_mut_ptr()), len);
        }
        self.len = len;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum FixedBufValue {
    Float(f64),
    Signed(i64),
    Unsigned(u64),
    String(FixedBufStr<64>),
    Bool(bool),
}

#[derive(Clone)]
pub enum Command {
    Project {
        app_name: FixedBufStr<64>,
        name: FixedBufStr<64>,
        version: FixedBufStr<64>,
    },

    SpanAlloc {
        id: u64,
        metadata: Meta,
    },

    SpanInit {
        span: u64,
        parent: Option<u64>, //None must mean that span is at root
    },

    SpanFollows {
        span: u64,
        follows: u64,
    },

    SpanValue {
        span: u64,
        key: &'static str,
        value: FixedBufValue
    },

    SpanMessage {
        span: u64,
        message: FixedBufStr<255>
    },

    Event {
        id: u32,
        timestamp: i64,
        message: LogMsg
    },

    SpanEnter(u64),

    SpanExit {
        span: u64,
        duration: Duration,
    },

    SpanFree(u64),

    Terminate,
}

fn read_command_line(payload: &mut nt::util::Payload) -> nt::header::Vchar {
    let mut r = payload.write_object("").unwrap();
    for v in std::env::args_os() {
        let head = payload.write_object(&*v.to_string_lossy()).unwrap();
        payload.write(b" ").unwrap();
        r.length += head.length + 1;
    }
    r
}

const OVERFLOW_LIMIT: u32 = 1_000_000_000;

struct SpanData {
    run_count: u32,
    instance_count: u32,
    has_overflowed: bool,
    min_time: Duration,
    max_time: Duration,
    total_time: Duration,
    parent: Option<u32>,
    name: &'static str,
    runs_file: Option<BufWriter<File>>
}

impl SpanData {
    pub fn new(name: &'static str, runs_file: Option<BufWriter<File>>) -> SpanData {
        SpanData {
            run_count: 0,
            instance_count: 0,
            has_overflowed: false,
            min_time: Duration::ZERO,
            max_time: Duration::MAX,
            total_time: Duration::ZERO,
            parent: None,
            name,
            runs_file
        }
    }

    pub fn get_average(&self) -> Duration {
        if self.run_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.run_count
        }
    }
}

struct SpanInstance {
    message_written: bool,
    csv_row: Cursor<[u8; 1024]>,
    variables: Cursor<[u8; 1024]>
}

impl SpanInstance {
    pub fn new() -> SpanInstance {
        SpanInstance {
            message_written: false,
            csv_row: Cursor::new([0; 1024]),
            variables: Cursor::new([0; 1024])
        }
    }

    pub fn write<T: Write>(&self, mut file: T) {
        let row_start = &self.csv_row.get_ref()[..self.csv_row.position() as _];
        let row_end = &self.variables.get_ref()[..self.variables.position() as _];
        let _ = file.write(row_start);
        let _ = file.write(row_end);
        let _ = file.write(b"\n");
    }

    pub fn finish(&mut self, duration: &Duration, name: &str) {
        if !self.message_written {
            let _ = write!(self.csv_row, "{}", name);
        }
        let _ = write!(self.csv_row, ",{},{},{}",
                       duration.as_secs(), duration.subsec_millis(),
                       duration.subsec_micros() - (duration.subsec_millis() * 1000));
    }

    pub fn append_value(&mut self, name: &'static str, value: &FixedBufValue) {
        let _ = match value {
            FixedBufValue::Float(v) => write!(self.variables, ",\"{}\"={}", name, v),
            FixedBufValue::Signed(v) => write!(self.variables, ",\"{}\"={}", name, v),
            FixedBufValue::Unsigned(v) => write!(self.variables, ",\"{}\"={}", name, v),
            FixedBufValue::String(v) => write!(self.variables, ",\"{}\"=\"{}\"", name, v.str()),
            FixedBufValue::Bool(v) => write!(self.variables, ",\"{}\"={}", name, v)
        };
    }

    pub fn append_message(&mut self, message: &FixedBufStr<255>) {
        let _ = write!(self.csv_row, "\"{}\"", message.str());
        self.message_written = true;
    }

    /*pub fn append(&mut self, buf: &[u8], message: Option<nt::header::Vchar>, value_set: Option<nt::header::Vchar>) {
        if let Some(msg) = message {
            if let Ok(str) = std::str::from_utf8(&buf[msg.offset as _..msg.length as _]) {
                let _ = write!(self.csv_row, "\"{}\"", str);
            }
            self.message_written = true;
        }
        if let Some(values) = value_set {
            let mut cursor = Cursor::new(&buf[values.offset as _..values.length as _]);
            while let Some(var) = nt::payload::Variable::from_buffer(&mut cursor) {
                let _ = match var.value {
                    nt::payload::Value::Float(v) => write!(self.variables, ",\"{}\"={}", var.name, v),
                    nt::payload::Value::Signed(v) => write!(self.variables, ",\"{}\"={}", var.name, v),
                    nt::payload::Value::Unsigned(v) => write!(self.variables, ",\"{}\"={}", var.name, v),
                    nt::payload::Value::String(v) => write!(self.variables, ",\"{}\"=\"{}\"", var.name, v),
                    nt::payload::Value::Bool(v) => write!(self.variables, ",\"{}\"={}", var.name, v),
                    _ => unreachable!()
                };
            }
        }
    }*/
}

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
