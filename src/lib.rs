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
use bp3d_fs::dirs::App;
use tracing::subscriber::set_global_default;
use crate::core::{Tracer, TracingSystem};
use crate::logger::Logger;
use crate::profiler::Profiler;

mod core;
mod util;
mod logger;
mod profiler;
mod visitor;

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
///
/// For simplified use, check `bp3d_tracing::setup!(app)`.
///
/// # Arguments
///
/// * `app`: the application name (ex: bp3d-sdk or bp3d-engine)
/// * `crate_name`: the name of the main root crate
/// * `crate_version`: the version of the main root crate
///
/// returns: Guard
pub fn initialize<T: AsRef<str>, T1: AsRef<str>, T2: AsRef<str>>(app: T, crate_name: T1, crate_version: T2) -> Guard {
    {
        let app = App::new(app.as_ref());
        if let Ok(v) = app.get_documents().map(|v| v.join("environment")) {
            bp3d_env::add_override_path(&v);
        }
    }
    let profiler = bp3d_env::get_bool("PROFILER").unwrap_or(false);
    if profiler {
        Profiler::new(app.as_ref(), crate_name.as_ref(), crate_version.as_ref()).map(load_system).unwrap_or_else(|_| load_system(Logger::new(app.as_ref())))
    } else {
        load_system(Logger::new(app.as_ref()))
    }
}

/// Initialize the logging and tracing systems with an application name.
/// Using this macro ensures the Guard structure is not dropped too early.
/// Additionally with this macro you don't have to pass in the crate name or version.
/// By default this macro will use CARGO_PKG_NAME and CARGO_PKG_VERSION to provide
/// the crate name and version.
///
/// # Example
///
/// ```
/// fn main() {
///     bp3d_tracing::setup!("my-super-app");
///     // ... application code goes here
/// }
/// ```
#[macro_export]
macro_rules! setup {
    ($app_name: expr) => {
        let _bp3d_tracing_guard = bp3d_tracing::initialize($app_name, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    };
}

static LOG_BUFFER_RC: AtomicUsize = AtomicUsize::new(0);

static STDOUT_DISABLE_RC: AtomicUsize = AtomicUsize::new(0);

/// A struct to automate management of the in-memory log buffer.
///
/// When a new instance of this struct is created, the log buffer is automatically enabled if not
/// already. Inversely, when all instances of this struct are dropped, the log buffer is disabled.
pub struct LogBuffer(()); //The empty type is used to force the use of the new function

impl LogBuffer {
    /// Creates a new access to the in-memory log buffer.
    pub fn new() -> LogBuffer {
        if LOG_BUFFER_RC.fetch_add(1, Ordering::Relaxed) == 0 {
            //If no previous buffers were created, enable the log buffer.
            bp3d_logger::enable_log_buffer();
        }
        LogBuffer(())
    }

    /// Attempts to pull a message from the in-memory log buffer.
    pub fn pull(&self) -> Option<bp3d_logger::LogMsg> {
        bp3d_logger::read_log()
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

/// A struct to automate enabling and disabling of the stdout/stderr logger.
///
/// When a new instance of this struct is created, the stdout/stderr logger is automatically
/// disabled if not already. Inversely, when all instances of this struct are dropped, the
/// stdout/stderr logger is re-enabled.
pub struct DisableStdoutLogger;

impl DisableStdoutLogger {
    /// Temporarily disables stdout/stderr logging for the lifespan of this struct.
    pub fn new() -> DisableStdoutLogger {
        if STDOUT_DISABLE_RC.fetch_add(1, Ordering::Relaxed) == 0 {
            //If no previous instances were created, disable the stdout/stderr logger.
            //First, flush any waiting message.
            bp3d_logger::flush();
            //Then disable the backend.
            bp3d_logger::disable_stdout();
        }
        DisableStdoutLogger
    }
}

impl Drop for DisableStdoutLogger {
    fn drop(&mut self) {
        if STDOUT_DISABLE_RC.fetch_sub(1, Ordering::Relaxed) == 1 {
            //If no more instances exists after this one, re-enable the stdout/stderr logger.
            bp3d_logger::enable_stdout();
        }
    }
}
