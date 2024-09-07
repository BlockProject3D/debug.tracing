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

use crate::profiler::network::hello::Packet;

const SIGNATURE: [u8; 8] = *b"BP3DPROF";

/*
struct Version {
    offset 0 major: u64
    offset 8 pre_release: [u8; 24] //0 padded
} size 32

struct Hello {
    offset 0 signature: [u8; 8]
    offset 8 version: Version
} size 40
*/

include!(concat!(env!("OUT_DIR"), "/version_inject.rs"));

pub enum MatchResult {
    SignatureMismatch,
    VersionMismatch,
    Ok,
}

impl<T: AsMut<[u8]>> Packet<T> {
    pub fn fill(&mut self) {
        self.get_signature_mut().as_mut().copy_from_slice(&SIGNATURE);
        self.get_version_mut().set_major(MAJOR_VERSION);
        self.get_version_mut().get_pre_release_mut().as_mut().copy_from_slice(&VERSION_DATA);
    }
}

impl<T: AsRef<[u8]>> Packet<T> {
    pub fn matches<T1: AsRef<[u8]>>(&self, other: &Packet<T1>) -> MatchResult {
        if self.get_signature().as_ref() != other.get_signature().as_ref() {
            return MatchResult::SignatureMismatch;
        }
        let val = match (self.get_version().get_pre_release().as_ref(), other.get_version().get_pre_release().as_ref()) {
            (b"", b"") => self.get_version().get_major() == other.get_version().get_major(),
            (a, b) => a == b
        };
        match val {
            true => MatchResult::Ok,
            false => MatchResult::VersionMismatch,
        }
    }
}
