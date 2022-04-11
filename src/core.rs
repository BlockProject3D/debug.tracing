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

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tracing_core::{Event, Metadata, Subscriber};
use tracing_core::span::{Attributes, Current, Id, Record};
use crate::util::hash_static_ref;

//TODO: Check if by any chance anything could panic (normally nothing should ever be able to panic here).

pub type Meta = &'static Metadata<'static>;

pub trait Tracer {
    fn enabled(&self, metadata: &Metadata) -> bool;
    fn span_create(&self, new: bool, parent: Option<Id>, span: &Attributes);
    fn span_values(&self, id: &Id, values: &Record);
    fn span_follows_from(&self, id: &Id, follows: &Id);
    fn event(&self, parent: Option<Id>, time: OffsetDateTime, event: &Event);
    fn span_enter(&self, id: &Id);
    fn span_exit(&self, id: &Id, duration: Duration);
    fn span_destroy(&self, id: Id);
}

struct SpanData {
    ref_count: usize,
    metadata: Meta,
    last_time: Option<Instant>
}

struct Inner {
    spans_by_meta: HashMap<usize, Id>,
    spans_by_id: HashMap<Id, SpanData>,
    current_span: Vec<Id>
}

pub struct BaseTracer<T>
{
    inner: Mutex<Inner>,
    counter: AtomicU64,
    derived: T
}

impl<T: 'static + Tracer> Subscriber for BaseTracer<T> {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.derived.enabled(metadata)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let (new, span_id) = {
            let mut lock = self.inner.lock().unwrap();
            match lock.spans_by_meta.get(&hash_static_ref(span.metadata().callsite().0)) {
                Some(v) => {
                    // The 2 clones are required otherwise the borrow checker refuses it
                    // even if spans_by_id is not the same field than spans_by_meta.
                    let v1 = v.clone();
                    let v2 = v.clone();
                    let data = lock.spans_by_id.get_mut(&v1).unwrap();
                    data.ref_count = 1;
                    (false, v2)
                }, //Why the fuck doesn't Id implement Copy? It's a fucking u64 so it should be copy fucking hell!
                None => {
                    //We're only ever fetch_add on the counter so no worries.
                    let new_id = Id::from_u64(self.counter.fetch_add(1, Ordering::Relaxed));
                    lock.spans_by_meta.insert(hash_static_ref(span.metadata().callsite().0), new_id.clone());
                    lock.spans_by_id.insert(new_id.clone(), SpanData {
                        metadata: span.metadata(),
                        ref_count: 1,
                        last_time: None
                    });
                    (true, new_id)
                }
            }
        };
        let parent = if span.is_root() {
            None
        } else {
            span.parent().cloned().or_else(|| {
                let lock = self.inner.lock().unwrap();
                lock.current_span.last().cloned()
            })
        };
        self.derived.span_create(new, parent, span);
        span_id
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.derived.span_values(span, values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.derived.span_follows_from(span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        let parent = {
            let lock = self.inner.lock().unwrap();
            lock.current_span.last().cloned()
        };
        self.derived.event(parent, OffsetDateTime::now_utc(), event);
    }

    fn enter(&self, span: &Id) {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(span) {
            data.last_time = Some(Instant::now());
            lock.current_span.push(span.clone());
            self.derived.span_enter(span);
        }
    }

    fn exit(&self, span: &Id) {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(span) {
            let duration = data.last_time.map(|v| v.elapsed())
                .unwrap_or_default();
            lock.current_span.retain(|v| v != span);
            self.derived.span_exit(span, duration);
        }
    }

    fn clone_span(&self, id: &Id) -> Id {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(id) {
            data.ref_count += 1;
        }
        id.clone()
    }

    fn try_close(&self, id: Id) -> bool {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(&id) {
            data.ref_count -= 1;
            if data.ref_count == 0 {
                self.derived.span_destroy(id);
                return true;
            }
        }
        false
    }

    fn current_span(&self) -> Current {
        let lock = self.inner.lock().unwrap();
        match lock.current_span.last() {
            Some(v) => Current::new(v.clone(), lock.spans_by_id[v].metadata),
            None => Current::none()
        }
    }
}
