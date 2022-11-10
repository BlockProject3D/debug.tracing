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

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::{Duration, Instant};
//use dashmap::DashMap;
use time::OffsetDateTime;
use tracing_core::{Event, Level, LevelFilter, Metadata, Subscriber};
use tracing_core::span::{Attributes, Current, Id, Record};
use crate::util::{hash_static_ref, Meta, span_from_id_instance, span_to_id_instance};

//TODO: Check if by any chance anything could panic (normally nothing should ever be able to panic here).

pub struct TracingSystem<T> {
    pub system: BaseTracer<T>,
    pub destructor: Option<Box<dyn Any>>
}

impl<T> TracingSystem<T> {
    pub fn with_destructor(derived: T, destructor: Box<dyn Any>) -> TracingSystem<T> {
        TracingSystem {
            system: BaseTracer::new(derived),
            destructor: Some(destructor)
        }
    }
}

pub trait Tracer {
    fn enabled(&self) -> bool;
    fn span_create(&self, id: &Id, new: bool, parent: Option<Id>, span: &Attributes);
    fn span_values(&self, id: &Id, values: &Record);
    fn span_follows_from(&self, id: &Id, follows: &Id);
    fn event(&self, parent: Option<Id>, time: OffsetDateTime, event: &Event);
    fn span_enter(&self, id: &Id);
    fn span_exit(&self, id: &Id, duration: Duration);
    fn span_destroy(&self, id: &Id);
    fn max_level_hint(&self) -> Option<Level>;
}

struct SpanData {
    ref_count: usize,
    metadata: Meta,
    last_time: Option<Instant>
}

struct SpanHead {
    span_id: u32,
    next_instance_id: u32,
    instance_count: u32,
    freed_instances: VecDeque<u32>
}

impl SpanHead {
    pub fn new(span_id: u32) -> SpanHead {
        SpanHead {
            span_id,
            next_instance_id: 0,
            instance_count: 0,
            freed_instances: VecDeque::new()
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
            },
            Some(v) => v
        }
    }
}

struct Inner {
    spans_by_meta: HashMap<usize, SpanHead>,
    spans_by_id: HashMap<Id, SpanData>,
    current_span_for_thread: HashMap<ThreadId, Vec<Id>>
}

impl Inner {
    pub fn new() -> Inner {
        Inner {
            spans_by_meta: HashMap::new(),
            spans_by_id: HashMap::new(),
            current_span_for_thread: HashMap::new()
        }
    }

    pub fn push_span(&mut self, id: &Id) {
        self.current_span_for_thread
            .entry(std::thread::current().id())
            .or_default()
            .push(id.clone());
    }

    pub fn pop_span(&mut self, id: &Id) {
        let current = &std::thread::current().id();
        if let Some(data) = self.current_span_for_thread.get_mut(current) {
            data.retain(|v| v != id);
            if data.len() == 0 {
                self.current_span_for_thread.remove(current);
            }
        }
    }

    pub fn current_span(&self) -> Option<Id> {
        self.current_span_for_thread
            .get(&std::thread::current().id())
            .map(|v| v.last().cloned())
            .flatten()
    }
}

pub struct BaseTracer<T> {
    inner: Mutex<Inner>,
    counter: AtomicU32,
    derived: T
}

impl<T> BaseTracer<T> {
    pub fn new(derived: T) -> BaseTracer<T> {
        BaseTracer {
            inner: Mutex::new(Inner::new()),
            counter: AtomicU32::new(1),
            derived
        }
    }
}

impl<T: 'static + Tracer> Subscriber for BaseTracer<T> {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if let Some(level) = self.derived.max_level_hint() {
            if level < *metadata.level() { //Levels compare at inverse logic!
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
            match lock.spans_by_meta.get_mut(&hash_static_ref(span.metadata().callsite().0)) {
                Some(v) => {
                    let instance = v.new_instance();
                    (false, span_from_id_instance(v.span_id, instance))
                }, //Why the fuck doesn't Id implement Copy? It's a fucking u64 so it should be copy fucking hell!
                None => {
                    //We're only ever fetch_add on the counter so no worries.
                    let span_id = self.counter.fetch_add(1, Ordering::Relaxed);
                    let mut head = SpanHead::new(span_id);
                    let instance = head.new_instance();
                    lock.spans_by_meta.insert(hash_static_ref(span.metadata().callsite().0), head);
                    (true, span_from_id_instance(span_id, instance))
                }
            }
        };
        let parent = if span.is_root() {
            None
        } else {
            span.parent().cloned().or_else(|| lock.current_span())
        };
        lock.spans_by_id.insert(span_id.clone(), SpanData {
            metadata: span.metadata(),
            ref_count: 1,
            last_time: None
        });
        self.derived.span_create(&span_id, new, parent, span);
        span_id
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.derived.span_values(span, values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.derived.span_follows_from(span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        self.derived.event(self.inner.lock().unwrap().current_span(), OffsetDateTime::now_utc(), event);
    }

    fn enter(&self, span: &Id) {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(span) {
            data.last_time = Some(Instant::now());
            lock.push_span(span);
            self.derived.span_enter(span);
        }
    }

    fn exit(&self, span: &Id) {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(span) {
            let duration = data.last_time.map(|v| v.elapsed())
                .unwrap_or_default();
            lock.pop_span(span);
            self.derived.span_exit(span, duration);
        }
    }

    fn clone_span(&self, id: &Id) -> Id {
        let mut lock = self.inner.lock().unwrap();
        if let Some(mut data) = lock.spans_by_id.get_mut(id) {
            data.ref_count += 1;
        }
        id.clone()
    }

    fn try_close(&self, id: Id) -> bool {
        let mut lock = self.inner.lock().unwrap();
        if let Some(data) = lock.spans_by_id.get_mut(&id) {
            data.ref_count -= 1;
            if data.ref_count == 0 {
                {
                    let fuckrust = data.metadata.callsite().0;
                    let head = lock.spans_by_meta.get_mut(&hash_static_ref(fuckrust)).unwrap();
                    let (_, instance) = span_to_id_instance(&id);
                    head.free_instance(instance);
                }
                lock.spans_by_id.remove(&id);
                self.derived.span_destroy(&id);
                return true;
            }
        }
        false
    }

    fn current_span(&self) -> Current {
        let lock = self.inner.lock().unwrap();
        match lock.current_span() {
            Some(v) => Current::new(v.clone(), lock.spans_by_id.get(&v).unwrap().metadata),
            None => Current::none()
        }
    }
}
