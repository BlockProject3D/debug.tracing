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

use super::model::{Color, Console, File, Level, LogQueue, Logger, Mode, Profiler};

// The default mode of the debugger.
pub const DEFAULT_MODE: Mode = Mode::Logger;

// The default color mode for the local logger implementation.
pub const DEFAULT_COLOR: Color = Color::Auto;

// The default log level.
pub const DEFAULT_LEVEL: Level = Level::Debug;

// The default port for the remote debugger.
pub const DEFAULT_PORT: u16 = 4026;

// The default maximum number of rows to be stored in the memory buffer.
pub const DEFAULT_MAX_ROWS: u32 = 1000000;

// Whether by default the local logger will use both stdout and stderr.
pub const DEFAULT_STDERR: bool = true;

// The default minimum period at which to send profiler updates.
pub const DEFAULT_MIN_PERIOD: u16 = 200;

// The default maximum count of log messages in the channel for the remote debugger.
pub const DEFAULT_REMOTE_BUF_SIZE: usize = 256;

// The default maximum count of log messages in the channel for the logger debugger.
pub const DEFAULT_LOGGER_BUF_SIZE: usize = 16;

// The default maximum count of log messages in the logger queue.
pub const DEFAULT_LOGGER_QUEUE_BUF_SIZE: usize = 16;

pub const DEFAULT_LOGGER: Logger = Logger {
    level: None,
    console: Some(Console {
        color: None,
        stderr: None,
        enabled: None
    }),
    file: Some(File {
        enabled: None
    }),
    buf_size: None,
    queue: None
};

pub const DEFAULT_PROFILER: Profiler = Profiler {
    port: None,
    max_rows: None,
    min_period: None,
    buf_size: None
};

pub const DEFAULT_CONSOLE: Console = Console {
    enabled: None,
    color: None,
    stderr: None
};

pub const DEFAULT_LOG_QUEUE: LogQueue = LogQueue {
    enabled: None,
    buf_size: None,
};

pub const DEFAULT_FILE: File = File {
    enabled: None,
};
