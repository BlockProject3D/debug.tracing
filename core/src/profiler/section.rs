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
use std::sync::{OnceLock};
use std::time::Instant;
use bp3d_logger::Location;
use crate::field::FieldSet;
use crate::profiler::{Profiler, profiler_section_record, profiler_section_register};

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

pub struct Entered<'a> {
    id: NonZeroU32,
    start: u64,
    fields: FieldSet<'a>
}

impl<'a> Drop for Entered<'a> {
    fn drop(&mut self) {
        let end = CUR_TIME.with(|v| v.elapsed().as_nanos() as _);
        unsafe { profiler_section_record(self.id, self.start, end, &self.fields) };
    }
}

pub struct Section {
    name: &'static str,
    location: Location,
    level: Level,
    parent: Option<&'static Section>,
    id: OnceLock<NonZeroU32>
}

impl Section {
    pub const fn new(name: &'static str, location: Location, level: Level) -> Self {
        Self {
            name,
            location,
            level,
            parent: None,
            id: OnceLock::new()
        }
    }

    pub const fn set_parent(mut self, parent: &'static Section) -> Self {
        self.parent = Some(parent);
        self
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

    pub fn parent(&self) -> Option<&'static Section> {
        self.parent
    }

    pub fn get_id(&'static self) -> &NonZeroU32 {
        self.id.get_or_init(|| unsafe { profiler_section_register(self) })
    }

    pub fn enter<'a>(&'static self, fields: FieldSet<'a>) -> Entered<'a> {
        let id = self.get_id();
        Entered {
            id: *id,
            start: CUR_TIME.with(|v| v.elapsed().as_nanos() as _),
            fields
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{field, fields, location};
    use crate::profiler::profiler_section_register;
    use crate::profiler::section::{Level, Section};

    #[test]
    fn basic() {
        static SECTION: Section = Section::new("api_test", location!(), Level::Event);
        //unsafe { profiler_section_register(&SECTION) };
    }

    #[test]
    fn api_test() {
        static SECTION: Section = Section::new("api_test", location!(), Level::Event);
        static SECTION2: Section = Section::new("api_test2", location!(), Level::Event)
            .set_parent(&SECTION);
        /*assert!(SECTION.enter([]).is_none());
        assert!(SECTION.enter(("test", 42)).is_none());
        assert!(SECTION.enter(("test", "test 123")).is_none());
        assert!(SECTION.enter(("test", 42.42)).is_none());
        assert!(SECTION.enter(("test", Level::Event)).is_none());
        assert!(SECTION.enter((("test", Level::Event), ("test2", 42))).is_none());*/
        let value = 32;
        let str = "this is a test";
        let lvl = Level::Event;
        SECTION.enter(fields!({value} {str} {?lvl} {test = value}));
    }
}
