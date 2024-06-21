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

use std::num::NonZeroU32;
use std::sync::OnceLock;
use crate::core::field::FieldSet;
use crate::core::logger::Logger;
use crate::core::profiler::Profiler;
use crate::core::types::MetadataRef;

pub struct Engine {

}

impl Profiler for Engine {
    fn section_register(&self, metadata: MetadataRef) -> NonZeroU32 {
        todo!()
    }

    fn section_create(&self, id: NonZeroU32) {
        todo!()
    }

    fn section_follows(&self, id: NonZeroU32, follows: NonZeroU32) {
        todo!()
    }

    fn section_exit<F: FieldSet>(&self, id: NonZeroU32, start: u64, end: u64, fields: F) {
        todo!()
    }
}

impl Logger for Engine {
    fn log_msg<F: FieldSet>(&self, metadata: MetadataRef, fields: F) {
        todo!()
    }
}

pub static ENGINE: OnceLock<Engine> = OnceLock::new();
