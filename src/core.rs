// Copyright (c) 2023, BlockProject 3D
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

use crate::util::{hash_static_ref, Meta, SpanId};
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing_core::span::{Attributes, Current, Id, Record};
use tracing_core::{Event, Level, LevelFilter, Metadata, Subscriber};

//TODO: Check if by any chance anything could panic (normally nothing should ever be able to panic here).

pub struct TracingSystem<T> {
    pub system: BaseTracer<T>,
    pub destructor: Option<Box<dyn Any>>,
}

impl<T> TracingSystem<T> {
    pub fn with_destructor(derived: T, destructor: Box<dyn Any>) -> TracingSystem<T> {
        TracingSystem {
            system: BaseTracer::new(derived),
            destructor: Some(destructor),
        }
    }
}

pub trait Tracer {
    fn enabled(&self) -> bool;
    fn span_create(&self, id: &SpanId, new: bool, parent: Option<SpanId>, span: &Attributes);
    fn span_values(&self, id: &SpanId, values: &Record);
    fn span_follows_from(&self, id: &SpanId, follows: &SpanId);
    fn event(&self, parent: Option<SpanId>, event: &Event);
    fn span_enter(&self, id: &SpanId);
    fn span_exit(&self, id: &SpanId, duration: Duration);
    fn span_destroy(&self, id: &SpanId);
    fn max_level_hint(&self) -> Option<Level>;
}

struct SpanData {
    ref_count: usize,
    metadata: Meta
}

struct SpanHead {
    span_id: NonZeroU32,
    next_instance_id: u32,
    instance_count: u32,
    freed_instances: VecDeque<u32>,
}

impl SpanHead {
    pub fn new(span_id: NonZeroU32) -> SpanHead {
        SpanHead {
            span_id,
            next_instance_id: 0,
            instance_count: 0,
            freed_instances: VecDeque::new(),
        }
    }

    pub fn free_instance(&mut self, id: u32) {
        self.instance_count -= 1;
        if self.instance_count == 0 {
            self.freed_instances.clear();
            self.next_instance_id = 0;
        } else {
            self.freed_instances.push_back(id);
        }
    }

    pub fn new_instance(&mut self) -> u32 {
        self.instance_count += 1;
        match self.freed_instances.pop_back() {
            None => {
                let id = self.next_instance_id;
                self.next_instance_id += 1;
                id
            }
            Some(v) => v,
        }
    }
}

struct Inner {
    spans_by_meta: HashMap<usize, SpanHead>,
    spans_by_id: HashMap<SpanId, SpanData>,
}

impl Inner {
    pub fn new() -> Inner {
        Inner {
            spans_by_meta: HashMap::new(),
            spans_by_id: HashMap::new(),
        }
    }
}

struct SpanLocal {
    id: SpanId,
    last_time: Instant
}

impl SpanLocal {
    pub fn new(id: SpanId) -> SpanLocal {
        SpanLocal {
            id,
            last_time: Instant::now()
        }
    }
}

thread_local! {
    static SPAN_STACK: RefCell<Vec<SpanLocal>> = RefCell::new(Vec::new());
}

#[inline]
fn push_span(span: SpanId) {
    SPAN_STACK.with(|v| {
        let mut stack = v.borrow_mut();
        stack.push(SpanLocal::new(span));
    });
}

#[inline]
fn pop_span(span: SpanId) -> Option<Duration> {
    SPAN_STACK.with(|v| {
        let mut stack = v.borrow_mut();
        let id = stack.iter().position(|v| v.id == span)?;
        let instant = stack[id].last_time;
        stack.remove(id);
        Some(instant.elapsed())
    })
}

#[inline]
fn current_span() -> Option<SpanId> {
    SPAN_STACK.with(|v| {
        let stack = v.borrow();
        stack.last().map(|v| v.id.clone())
    })
}

pub struct BaseTracer<T> {
    inner: Mutex<Inner>,
    counter: AtomicU32,
    derived: T,
}

impl<T> BaseTracer<T> {
    pub fn new(derived: T) -> BaseTracer<T> {
        BaseTracer {
            inner: Mutex::new(Inner::new()),
            counter: AtomicU32::new(1),
            derived,
        }
    }
}

impl<T: 'static + Tracer> Subscriber for BaseTracer<T> {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if let Some(level) = self.derived.max_level_hint() {
            if level < *metadata.level() {
                //Levels compare at inverse logic!
                return false;
            }
        }
        self.derived.enabled()
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.derived.max_level_hint().map(LevelFilter::from_level)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut lock = self.inner.lock().unwrap();
        let (new, span_id) = {
            match lock
                .spans_by_meta
                .get_mut(&hash_static_ref(span.metadata().callsite().0))
            {
                Some(v) => {
                    let instance = v.new_instance();
                    (false, SpanId::from((v.span_id, instance)))
                } //Why the fuck doesn't Id implement Copy? It's a fucking u64 so it should be copy fucking hell!
                None => {
                    //We're only ever fetch_add on the counter so no worries.
                    let span_id = self.counter.fetch_add(1, Ordering::Relaxed);
                    //SAFETY: The counter is initialized at 1 so fetch_add cannot return 0.
                    let span_id = unsafe { NonZeroU32::new_unchecked(span_id) };
                    let mut head = SpanHead::new(span_id);
                    let instance = head.new_instance();
                    lock.spans_by_meta
                        .insert(hash_static_ref(span.metadata().callsite().0), head);
                    (true, SpanId::from((span_id, instance)))
                }
            }
        };
        let parent = if span.is_root() {
            None
        } else {
            span.parent().map(SpanId::from).or_else(current_span)
        };
        lock.spans_by_id.insert(
            span_id,
            SpanData {
                metadata: span.metadata(),
                ref_count: 1
            },
        );
        self.derived.span_create(&span_id, new, parent, span);
        span_id.into_id()
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.derived.span_values(&span.into(), values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.derived
            .span_follows_from(&span.into(), &follows.into());
    }

    fn event(&self, event: &Event<'_>) {
        self.derived.event(current_span(), event);
    }

    fn enter(&self, span: &Id) {
        let span = span.into();
        push_span(span);
        self.derived.span_enter(&span);
    }

    fn exit(&self, span: &Id) {
        let span = span.into();
        if let Some(duration) = pop_span(span) {
            self.derived.span_exit(&span, duration);
        }
    }

    fn clone_span(&self, id: &Id) -> Id {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(&id.into()) {
            data.ref_count += 1;
        }
        id.clone()
    }

    fn try_close(&self, id: Id) -> bool {
        let span = id.into();
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(&span) {
            data.ref_count -= 1;
            if data.ref_count == 0 {
                {
                    let fuckrust = data.metadata.callsite().0;
                    let head = lock
                        .spans_by_meta
                        .get_mut(&hash_static_ref(fuckrust))
                        .unwrap();
                    head.free_instance(span.get_instance());
                }
                lock.spans_by_id.remove(&span);
                self.derived.span_destroy(&span);
                return true;
            }
        }
        false
    }

    fn current_span(&self) -> Current {
        match current_span() {
            None => Current::none(),
            Some(v) => {
                let lock = self.inner.lock().unwrap();
                Current::new(v.into_id(), lock.spans_by_id.get(&v).unwrap().metadata)
            }
        }
    }
}
