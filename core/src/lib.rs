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

use bp3d_os::dirs::App;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::core2::Terminate;
use crate::debug_logger::{LocalDebugger, LOCAL_DEBUGGER};
use crate::profiler::{RemoteDebugger, REMOTE_DEBUGGER};

mod config;
mod tracer_base;
mod debug_logger;
//mod core;
//mod logger;
mod profiler;
mod core2;
//mod util;
//mod visitor;

/// The guard to ensure proper termination of logging and tracing systems.
pub struct Guard(Option<&'static dyn Terminate>);

impl Guard {
    /// Run the following closure then terminate logging and tracing systems.
    pub fn run<R, F: FnOnce() -> R>(self, func: F) -> R {
        func()
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        match self.0 {
            Some(v) => v.terminate(),
            _ => ()
        }
    }
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
pub fn initialize<T: AsRef<str>, T1: AsRef<str>, T2: AsRef<str>>(
    app: T,
    crate_name: T1,
    crate_version: T2,
) -> Guard {
    let config = {
        let app = App::new(app.as_ref());
        config::load_config(&app)
    };
    let profiler = bp3d_os::env::get_bool("PROFILER").unwrap_or(false);
    let disable = bp3d_os::env::get_bool("LOG_DISABLE").unwrap_or(false);
    if config.get_mode() == config::model::Mode::None || disable {
        Guard(None)
    } else if config.get_mode() == config::model::Mode::Profiler || profiler {
        let debugger: &'static dyn Terminate = match RemoteDebugger::new(app.as_ref(), crate_name.as_ref(), crate_version.as_ref(), &config) {
            Err(e) => {
                eprintln!("Failed to initialize profiler: {}", e);
                let stat = LOCAL_DEBUGGER.get_or_init(|| LocalDebugger::new(app.as_ref(), &config));
                bp3d_debug::engine::set(stat);
                stat
            },
            Ok(v) => {
                let stat = REMOTE_DEBUGGER.get_or_init(|| v);
                bp3d_debug::engine::set(stat);
                stat
            }
        };
        Guard(Some(debugger))
    } else {
        let debugger = LOCAL_DEBUGGER.get_or_init(|| LocalDebugger::new(app.as_ref(), &config));
        bp3d_debug::engine::set(debugger);
        Guard(Some(debugger))
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
        let _bp3d_tracing_guard =
            bp3d_tracing::initialize($app_name, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    };
}

static STDOUT_DISABLE_RC: AtomicUsize = AtomicUsize::new(0);

/// A struct to automate enabling and disabling of the stdout/stderr logger.
///
/// When a new instance of this struct is created, the stdout/stderr logger is automatically
/// disabled if not already. Inversely, when all instances of this struct are dropped, the
/// stdout/stderr logger is re-enabled.
pub struct DisableConsole;

impl DisableConsole {
    /// Temporarily disables stdout/stderr logging for the lifespan of this struct.
    pub fn new() -> DisableConsole {
        if STDOUT_DISABLE_RC.fetch_add(1, Ordering::Relaxed) == 0 {
            //If no previous instances were created, disable the stdout/stderr logger.
            if let Some(v) = LOCAL_DEBUGGER.get() {
                //First, flush any waiting message.
                v.flush();
                //Then disable the backend.
                v.enable_stdout(false);
            }
        }
        DisableConsole
    }
}

impl Drop for DisableConsole {
    fn drop(&mut self) {
        if STDOUT_DISABLE_RC.fetch_sub(1, Ordering::Relaxed) == 1 {
            //If no more instances exists after this one, re-enable the stdout/stderr logger.
            if let Some(v) = LOCAL_DEBUGGER.get() {
                v.enable_stdout(true);
            }
        }
    }
}
