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

use std::io::{ErrorKind, Error};

use crate::profiler::network_types as nt;
use serde::{Deserialize, Serialize};
use tokio::{io::{BufWriter, BufReader, AsyncReadExt, AsyncWriteExt}, net::{tcp::{WriteHalf, ReadHalf}, TcpStream}};

pub struct SmallPayload {
    cursor: usize,
    net_buffer: [u8; 1024]
}

impl SmallPayload {
    pub fn new() -> SmallPayload {
        SmallPayload {
            cursor: 0,
            net_buffer: [0; 1024]
        }
    }

    pub fn get_payload(&mut self) -> nt::util::Payload {
        self.cursor = 0;
        nt::util::Payload::new(&mut self.net_buffer, &mut self.cursor)
    }
}

pub struct Net<'a> {
    write: BufWriter<WriteHalf<'a>>,
    head_buffer: [u8; 64],
    read: BufReader<ReadHalf<'a>>,
}

pub const PAYLOAD_NONE: Option<[u8; 0]> = None;

impl<'a> Net<'a> {
    pub fn new(socket: &'a mut TcpStream) -> Net<'a> {
        let (read, write) = socket.split();
        Net {
            write: BufWriter::new(write),
            read: BufReader::new(read),
            head_buffer: [0; 64]
        }
    }

    pub async fn flush_raw(&mut self) -> std::io::Result<()> {
        self.write.flush().await
    }

    pub async fn flush(&mut self) {
        if let Err(e) = self.write.flush().await {
            eprintln!("Failed to write to network: {}", e);
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

    pub async fn network_write_raw<H: Serialize + nt::header::MsgHeader, P: AsRef<[u8]>>(
        &mut self,
        header: H,
        payload: Option<P>
    ) -> std::io::Result<()> {
        let mut head_len = 0;
        let mut serializer = nt::serializer::Serializer::new(&mut self.head_buffer, &mut head_len);
        if let Err(e) = H::TYPE.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        if let Err(e) = header.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        self.write.write_all(&self.head_buffer[..head_len]).await?;
        if let Some(payload) = payload {
            self.write
                .write_all(payload.as_ref())
                .await?;
        }
        Ok(())
    }

    pub async fn network_write<H: Serialize + nt::header::MsgHeader, P: AsRef<[u8]>>(&mut self, header: H, payload: Option<P>) {
        if let Err(e) = self.network_write_raw(header, payload).await {
            eprintln!("Failed to write to network: {}", e);
        }
    }
}
