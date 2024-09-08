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

use std::io::Cursor;
use bp3d_proto::message::{WriteSelf, WriteSelfAsync};
use bp3d_proto::util::FixedSize;
use crate::remote::network as net;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

pub struct Net<'a> {
    write: BufWriter<WriteHalf<'a>>,
    read: BufReader<ReadHalf<'a>>,
    fixed_buffer: [u8; 64],
}

impl<'a> Net<'a> {
    pub fn new(socket: &'a mut TcpStream) -> Net<'a> {
        let (read, write) = socket.split();
        Net {
            write: BufWriter::new(write),
            read: BufReader::new(read),
            fixed_buffer: [0; 64],
        }
    }

    pub async fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush().await
    }

    pub async fn network_read_fixed<'b, M: FixedSize + From<&'b [u8]>>(
        &'b mut self,
    ) -> std::io::Result<M> {
        self.read
            .read_exact(&mut self.fixed_buffer[0..M::SIZE])
            .await?;
        Ok(M::from(&self.fixed_buffer[0..M::SIZE]))
    }

    pub async fn network_write_fixed<M: FixedSize + AsRef<[u8]>>(
        &mut self,
        ty: net::message::Type,
        message: M,
    ) -> std::io::Result<()> {
        let mut msg = net::message::Header::new_on_stack();
        msg.set_type(ty).set_size(M::SIZE as _);
        self.write.write_all(msg.as_ref()).await?;
        self.write.write_all(message.as_ref()).await?;
        Ok(())
    }

    pub async fn network_write_dyn<'b, M: WriteSelf, B: AsMut<[u8]>>(
        &mut self,
        ty: net::message::Type,
        message: M,
        mut buffer: B,
    ) -> bp3d_proto::message::Result<()> {
        let mut cursor = Cursor::new(buffer.as_mut());
        message.write_self(&mut cursor)?;
        let mut msg = net::message::Header::new_on_stack();
        msg.set_type(ty).set_size(cursor.position() as _);
        self.write.write_all(msg.as_ref()).await?;
        let motherfuckingrust = cursor.position() as _;
        self.write.write_all(&buffer.as_mut()[..motherfuckingrust]).await.map_err(bp3d_proto::message::Error::Io)?;
        Ok(())
    }

    pub async fn network_write_dyn_payload<'b, M: WriteSelf + WriteSelfAsync>(&mut self, ty: net::message::Type, message: M) -> bp3d_proto::message::Result<()> {
        let mut msg = net::message::Header::new_on_stack();
        msg.set_type(ty).set_size(message.size()? as _);
        self.write.write_all(msg.as_ref()).await?;
        message.write_self_async(&mut self.write).await?;

        Ok(())
    }
}
