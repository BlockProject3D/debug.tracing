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
use std::sync::atomic::{AtomicUsize, Ordering};
use bp3d_logger::{GetLogs};
use tracing::subscriber::set_global_default;
use crate::core::{Tracer, TracingSystem};
use crate::logger::Logger;
use crate::profiler::Profiler;
use crate::util::check_env_bool;

mod core;
mod util;
mod logger;
mod profiler;

/// The guard to ensure proper termination of logging and tracing systems.
pub struct Guard(Option<Box<dyn Any>>);

impl Guard {
    /// Run the following closure then terminate logging and tracing systems.
    pub fn run<R, F: FnOnce() -> R>(self, func: F) -> R {
        func()
    }
}

fn load_system<T: 'static + Tracer + Sync + Send>(system: TracingSystem<T>) -> Guard {
    set_global_default(system.system).expect("bp3d-tracing can only be initialized once!");
    Guard(system.destructor)
}

/// Initialize the logging and tracing systems for the given application.
///
/// The function returns a guard which must be maintained for the duration of the application.
pub fn initialize<T: GetLogs>(app: T) -> Guard {
    let profiler = check_env_bool("PROFILER");
    if profiler {
        Profiler::new().map(load_system).unwrap_or(load_system(Logger::new(app)))
    } else {
        load_system(Logger::new(app))
    }
}

static LOG_BUFFER_RC: AtomicUsize = AtomicUsize::new(0);

/// A struct to automate management of the in-memory log buffer.
///
/// When a new instance of this struct is created, the log buffer is automatically enabled if not
/// already. Inversely, when all instances of this struct are dropped, the log buffer is disabled.
pub struct LogBuffer(bp3d_logger::LogBuffer);

impl LogBuffer {
    /// Creates a new access to the in-memory log buffer.
    pub fn new() -> LogBuffer {
        if LOG_BUFFER_RC.fetch_add(1, Ordering::Relaxed) == 0 {
            //If no previous buffers were created, enable the log buffer.
            bp3d_logger::enable_log_buffer();
        }
        LogBuffer(bp3d_logger::get_log_buffer())
    }

    /// Attempts to pull a message from the in-memory log buffer.
    pub fn pull(&self) -> Option<bp3d_logger::LogMsg> {
        self.0.try_recv().ok()
    }
}

impl Drop for LogBuffer {
    fn drop(&mut self) {
        if LOG_BUFFER_RC.fetch_sub(1, Ordering::Relaxed) == 1 {
            //If no more log buffers exists after this one, disable the log buffer.
            bp3d_logger::disable_log_buffer();
        }
    }
}
