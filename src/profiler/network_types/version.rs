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

//! This module contains the standard declarations for the protocol initialization. All BP3D
//! protocols (except auto-discovery for now) expose a Hello packet.

use byteorder::{ByteOrder, LittleEndian};

const SIGNATURE: [u8; 8] = *b"BP3DPROF";

//Follow semver except that we exclude the build metadata, the minor and the patch because only the
// major version records protocol changes.

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

pub enum MatchResult {
    SignatureMismatch,
    VersionMismatch,
    Ok
}

pub struct Version {
    major: u64,
    pre_release: Option<[u8; 24]>,
}

pub struct Hello {
    signature: [u8; 8],
    version: Version
}

impl Hello {
    pub const fn new(major: u64, pre_release: Option<[u8; 24]>) -> Self {
        Self {
            signature: SIGNATURE,
            version: Version {
                major,
                pre_release
            }
        }
    }

    pub fn from_bytes(block: [u8; 40]) -> Self {
        let mut signature: [u8; 8] = [0; 8];
        let mut pre_release: [u8; 24] = [0; 24];
        signature.copy_from_slice(&block[..8]);
        pre_release.copy_from_slice(&block[16..]);
        let major = LittleEndian::read_u64(&block[8..16]);
        if pre_release[0] == 0x0 {
            Hello {
                signature,
                version: Version {
                    major,
                    pre_release: None
                }
            }
        } else {
            Hello {
                signature,
                version: Version {
                    major,
                    pre_release: Some(pre_release)
                }
            }
        }
    }

    pub fn matches(&self, other: &Hello) -> MatchResult {
        if self.signature != other.signature {
            return MatchResult::SignatureMismatch;
        }
        let val = match (self.version.pre_release, other.version.pre_release) {
            (Some(a), Some(b)) => a == b,
            (None, None) => self.version.major == other.version.major,
            _ => false
        };
        match val {
            true => MatchResult::Ok,
            false => MatchResult::VersionMismatch
        }
    }

    pub fn to_bytes(&self) -> [u8; 40] {
        let mut block: [u8; 40] = [0; 40];
        block[..8].copy_from_slice(&self.signature);
        LittleEndian::write_u64(&mut block[8..16], self.version.major);
        if let Some(pre_release) = &self.version.pre_release {
            block[16..].copy_from_slice(pre_release);
        }
        block
    }
}

include!(concat!(env!("OUT_DIR"), "/version_inject.rs"));
