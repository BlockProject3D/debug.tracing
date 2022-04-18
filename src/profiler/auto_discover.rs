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

use std::io::Write;
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crate::profiler::{DEFAULT_PORT, PROTOCOL_VERSION};

// The maximum number of characters allowed for the application name in the auto-discover list.
const NAME_MAX_CHARS: usize = 126;

const PROTOCOL_SIGNATURE: u8 = b'B';

pub struct AutoDiscoveryService {
    socket: UdpSocket,
    packet: Box<[u8]>,
    exit_flag: Arc<AtomicBool>
}

impl AutoDiscoveryService {
    pub fn new(app_name: &str) -> std::io::Result<AutoDiscoveryService> {
        let bytes = app_name.as_bytes();
        let truncated = &bytes[..std::cmp::min(bytes.len(), NAME_MAX_CHARS)];
        let mut packet = Vec::with_capacity(NAME_MAX_CHARS + 2);
        packet.push(PROTOCOL_SIGNATURE);
        packet.push(PROTOCOL_VERSION);
        packet.write_all(truncated).unwrap();
        // Null-pad the packet.
        while packet.len() != NAME_MAX_CHARS + 2 {
            packet.push(0);
        }
        let exit_flag = Arc::new(AtomicBool::new(false));
        let socket = UdpSocket::bind((Ipv4Addr::new(0, 0, 0, 0), 0))?;
        socket.set_broadcast(true)?;
        Ok(AutoDiscoveryService {
            packet: packet.into_boxed_slice(),
            exit_flag,
            socket
        })
    }

    pub fn get_exit_flag(&self) -> Arc<AtomicBool> {
        self.exit_flag.clone()
    }

    pub fn run(&self) {
        loop {
            if self.exit_flag.load(Ordering::Relaxed) {
                break;
            }
            if let Err(e) = self.socket.send_to(&self.packet, (Ipv4Addr::BROADCAST, DEFAULT_PORT)) {
                eprintln!("Failed to send broadcast auto-discover packet: {}", e);
            }
            std::thread::sleep(Duration::from_secs(2)); //Broadcast ourself every 2 seconds.
        }
    }
}
