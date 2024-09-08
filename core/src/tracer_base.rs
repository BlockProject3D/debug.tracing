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

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use bp3d_debug::trace::span::{Callsite, Id};
use parking_lot::{Mutex, RawMutex, RwLock};
use parking_lot::lock_api::{MappedMutexGuard, MutexGuard};

type Guard<'a, T> = MappedMutexGuard<'a, RawMutex, SpanData<T>>;

pub struct SpanData<T> {
    order: u32,
    uses: u32,
    start: u64,
    end: u64,
    content: T
}

impl<T> Deref for SpanData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<T> DerefMut for SpanData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

impl<T> SpanData<T> {
    pub fn order(&self) -> u32 {
        self.order
    }

    pub fn uses(&self) -> u32 {
        self.uses
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }
}

struct SpanMap<T> {
    spans: Vec<SpanData<T>>,
    cur_order: u32
}

pub struct BaseTracer<T> {
    callsites: RwLock<HashMap<NonZeroU32, &'static Callsite>>,
    cur_callsite: AtomicU32,
    spans: Mutex<SpanMap<T>>,
    time: Instant
}

impl<T> BaseTracer<T> {
    pub fn new() -> Self {
        Self {
            callsites: RwLock::new(HashMap::new()),
            cur_callsite: AtomicU32::new(1),
            spans: Mutex::new(SpanMap {
                spans: Vec::new(),
                cur_order: 1
            }),
            time: Instant::now()
        }
    }

    pub fn register_callsite(&self, callsite: &'static Callsite) -> NonZeroU32 {
        let id = unsafe { NonZeroU32::new_unchecked(self.cur_callsite.fetch_add(1, Ordering::Relaxed)) };
        let mut guard = self.callsites.write();
        guard.insert(id, callsite);
        id
    }

    pub fn get_callsite(&self, id: NonZeroU32) -> &'static Callsite {
        let guard = self.callsites.read();
        guard[&id]
    }

    pub fn create_span(&self, content: T) -> (NonZeroU32, Guard<T>) {
        let mut guard = self.spans.lock();
        //TODO: warning this may bug if a slot is re-claimed because the content is not cleared
        let id = guard.spans.iter().enumerate().find_map(|(i, v)| match v.order {
            0 => Some(i),
            _ => None
        }).unwrap_or_else(|| {
            guard.spans.push(SpanData {
                order: 0,
                uses: 0,
                start: 0,
                end: 0,
                content
            });
            guard.spans.len() - 1
        });
        unsafe {
            guard.spans.get_unchecked_mut(id).order = guard.cur_order;
            guard.cur_order += 1;
            guard.spans.get_unchecked_mut(id).uses += 1;
        }
        let id1 = unsafe { NonZeroU32::new_unchecked((id + 1) as _) };
        (id1, MutexGuard::map(guard, |v| unsafe { v.spans.get_unchecked_mut(id) }))
    }

    pub fn get_data(&self, id: Id) -> Guard<T> {
        let guard = self.spans.lock();
        MutexGuard::map(guard, |v| unsafe { v.spans.get_unchecked_mut(id.get_instance().get() as usize) })
    }

    pub fn span_enter(&self, id: Id) -> Guard<T> {
        let mut data = self.get_data(id);
        data.uses += 1;
        data.start = self.time.elapsed().as_nanos() as _;
        data
    }

    pub fn span_exit(&self, id: Id) -> Guard<T> {
        let mut data = self.get_data(id);
        data.uses -= 1;
        data.end = self.time.elapsed().as_nanos() as _;
        if data.uses == 0 {
            data.order = 0;
        }
        data
    }

    pub fn destroy_span(&self, id: Id) {
        let mut data = self.get_data(id);
        data.uses -= 1;
        if data.uses == 0 {
            data.order = 0;
        }
    }
}
