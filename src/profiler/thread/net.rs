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

use std::io::{ErrorKind, Error, Cursor};

use crate::profiler::network_types as nt;
use serde::{Deserialize, Serialize};
use tokio::{io::{BufWriter, BufReader, AsyncReadExt, AsyncWriteExt}, net::{tcp::{WriteHalf, ReadHalf}, TcpStream}};

pub struct Net<'a> {
    write: BufWriter<WriteHalf<'a>>,
    read: BufReader<ReadHalf<'a>>,
    fixed_buffer: [u8; 64]
}

impl<'a> Net<'a> {
    pub fn new(socket: &'a mut TcpStream) -> Net<'a> {
        let (read, write) = socket.split();
        Net {
            write: BufWriter::new(write),
            read: BufReader::new(read),
            fixed_buffer: [0; 64]
        }
    }

    pub async fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush().await
    }

    pub async fn network_read_fixed<'b, M: nt::message::MsgSize + Deserialize<'b>>(
        &'b mut self,
    ) -> std::io::Result<M> {
        self.read
            .read_exact(&mut self.fixed_buffer[0..M::SIZE])
            .await?;
        let mut de = nt::deserializer::Deserializer::new(&self.fixed_buffer[0..M::SIZE]);
        M::deserialize(&mut de).map_err(|e| Error::new(ErrorKind::Other, e))
    }

    pub async fn network_write_fixed<M: Serialize + nt::message::MsgSize + nt::message::Msg>(
        &mut self,
        message: M
    ) -> std::io::Result<()> {
        let mut cursor = Cursor::new(&mut self.fixed_buffer as &mut [u8]);
        let mut serializer = nt::serializer::Serializer::new(&mut cursor);
        if let Err(e) = M::TYPE.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        if let Err(e) = message.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        self.write.write_u32_le(M::SIZE as _).await?;
        self.write.write_all(&self.fixed_buffer[..M::SIZE]).await?;
        Ok(())
    }

    pub async fn network_write_dyn<M: Serialize + nt::message::Msg, B: AsMut<[u8]>>(
        &mut self,
        message: M,
        mut buffer: B
    ) -> std::io::Result<()> {
        let mut cursor = Cursor::new(buffer.as_mut());
        let mut serializer = nt::serializer::Serializer::new(&mut cursor);
        if let Err(e) = M::TYPE.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        if let Err(e) = message.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        self.write.write_u32_le(cursor.position() as _).await?;
        let motherfuckingrust = cursor.position() as usize;
        self.write.write_all(&buffer.as_mut()[..motherfuckingrust]).await?;
        Ok(())
    }

    pub async fn network_write_fixed_payload<M: Serialize + nt::message::Msg + nt::message::MsgSize, B: AsRef<[u8]>>(
        &mut self,
        message: M,
        buffer: B
    ) -> std::io::Result<()> {
        let mut cursor = Cursor::new(&mut self.fixed_buffer as &mut [u8]);
        let mut serializer = nt::serializer::Serializer::new(&mut cursor);
        if let Err(e) = M::TYPE.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        if let Err(e) = message.serialize(&mut serializer) {
            return Err(Error::new(ErrorKind::Other, e));
        }
        self.write.write_u32_le((M::SIZE + buffer.as_ref().len()) as _).await?;
        self.write.write_all(&self.fixed_buffer[..M::SIZE]).await?;
        self.write.write_all(buffer.as_ref()).await?;
        Ok(())
    }
}
