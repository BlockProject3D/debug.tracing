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
use bp3d_logger::Location;
use crate::field::FieldSet;
use crate::trace::Tracer;

pub struct Callsite {
    name: &'static str,
    location: Location,
    id: OnceLock<Option<NonZeroU32>>
}

impl Callsite {
    pub const fn new(name: &'static str, location: Location) -> Self {
        Self {
            name,
            location,
            id: OnceLock::new()
        }
    }

    pub fn get_id(&'static self) -> &Option<NonZeroU32> {
        self.id.get_or_init(|| crate::core::ENGINE.get().map(|v| v.register_callsite(self)))
    }
}

pub struct Span {
    id: Option<NonZeroU32>
}

impl Span {
    pub fn new<F: FieldSet>(callsite: &'static Callsite, fields: &F) -> Self {
        let id = callsite.get_id()
            .map(|cid| crate::core::ENGINE.get().map(|v| v.span_create(cid, fields)))
            .flatten();
        Self {
            id
        }
    }

    pub fn record<F: FieldSet>(&self, fields: &F) {
        if let Some(id) = self.id {
            unsafe { crate::core::ENGINE.get().unwrap_unchecked().span_record(id, fields) };
        }
    }

    pub fn enter(&self) {
        if let Some(id) = self.id {
            unsafe { crate::core::ENGINE.get().unwrap_unchecked().span_enter(id) };
        }
    }
}

impl Drop for Span  {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            unsafe { crate::core::ENGINE.get().unwrap_unchecked().span_exit(id) };
        }
    }
}
