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
use std::sync::{OnceLock};
use std::time::Instant;
use bp3d_logger::Location;
use crate::field::FieldSet;
use crate::profiler::Profiler;

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Level {
    /// A section located in a critically hot path.
    Critical = 0,

    /// A periodic section.
    Periodic = 1,

    // An event based section.
    Event = 2
}

thread_local! {
    static CUR_TIME: Instant = Instant::now();
}

pub struct Entered<F: FieldSet> {
    id: NonZeroU32,
    start: u64,
    fields: ManuallyDrop<F>
}

impl<F: FieldSet> Drop for Entered<F> {
    fn drop(&mut self) {
        let end = CUR_TIME.with(|v| v.elapsed().as_nanos() as _);
        let engine = unsafe { crate::core::ENGINE.get().unwrap_unchecked() };
        let fields = unsafe { ManuallyDrop::into_inner(std::ptr::read(&self.fields)) };
        engine.section_exit(self.id, self.start, end, fields);
    }
}

pub struct Section {
    name: &'static str,
    location: Location,
    level: Level,
    id: OnceLock<Option<NonZeroU32>>
}

impl Section {
    pub const fn new(name: &'static str, location: Location, level: Level) -> Self {
        Self {
            name,
            location,
            level,
            id: OnceLock::new()
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn enter<F: FieldSet>(&'static self, fields: F) -> Option<Entered<F>> {
        let id = self.id.get_or_init(|| crate::core::ENGINE.get().map(|v| v.section_register(self)));
        id.map(|id| Entered {
            id,
            start: CUR_TIME.with(|v| v.elapsed().as_nanos() as _),
            fields: ManuallyDrop::new(fields)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{field, fields, location};
    use crate::field::D;
    use crate::profiler::section::{Level, Section};

    #[test]
    fn api_test() {
        static SECTION: Section = Section::new("api_test", location!(), Level::Event);
        assert!(SECTION.enter(()).is_none());
        assert!(SECTION.enter(("test", 42)).is_none());
        assert!(SECTION.enter(("test", "test 123")).is_none());
        assert!(SECTION.enter(("test", 42.42)).is_none());
        assert!(SECTION.enter(("test", D(Level::Event))).is_none());
        assert!(SECTION.enter((("test", D(Level::Event)), ("test2", 42))).is_none());
        let value = 32;
        let str = "this is a test";
        let lvl = Level::Event;
        assert!(SECTION.enter(fields!({value} {str} {?lvl} {test = value})).is_none());
    }
}
