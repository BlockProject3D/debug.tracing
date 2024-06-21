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

use std::mem::ManuallyDrop;
use std::num::NonZeroU32;
use std::time::Instant;
use crate::core::engine::ENGINE;
use crate::core::field::FieldSet;
use crate::core::types::{Metadata, MetadataRef};

thread_local! {
    static CUR_TIME: Instant = Instant::now();
}

pub struct EnteredSection<F: FieldSet> {
    id: NonZeroU32,
    start: u64,
    fields: ManuallyDrop<F>
}

impl<F: FieldSet> Drop for EnteredSection<F> {
    fn drop(&mut self) {
        let engine = unsafe { ENGINE.get().unwrap_unchecked() };
        let end = CUR_TIME.with(|v| v.elapsed().as_nanos() as _);
        let fields = unsafe { ManuallyDrop::into_inner(std::ptr::read(&self.fields)) };
        engine.section_exit(self.id, self.start, end, fields);
    }
}

pub struct ProfilerSection {
    id: Option<NonZeroU32>
}

impl ProfilerSection {
    pub fn new(metadata: &'static Metadata) -> Self {
        let id = ENGINE.get().map(|engine| engine.section_register(MetadataRef::Borrowed(metadata)));
        Self {
            id,
        }
    }

    pub fn enter<F: FieldSet>(&self, fields: F) -> Option<EnteredSection<F>> {
        self.id.map(|id| EnteredSection {
            id,
            start: CUR_TIME.with(|v| v.elapsed().as_nanos() as _),
            fields: ManuallyDrop::new(fields)
        })
    }
}

pub trait Profiler {
    fn section_register(&self, metadata: MetadataRef) -> NonZeroU32;
    fn section_create(&self, id: NonZeroU32);
    fn section_follows(&self, id: NonZeroU32, follows: NonZeroU32);
    fn section_exit<F: FieldSet>(&self, id: NonZeroU32, start: u64, end: u64, fields: F);
}
