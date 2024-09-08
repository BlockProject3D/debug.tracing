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

use serde::Deserialize;

use super::defaults::{DEFAULT_REMOTE_BUF_SIZE, DEFAULT_COLOR, DEFAULT_CONSOLE, DEFAULT_LEVEL, DEFAULT_LOGGER, DEFAULT_MAX_ROWS, DEFAULT_MIN_PERIOD, DEFAULT_MODE, DEFAULT_PORT, DEFAULT_PROFILER, DEFAULT_STDERR, DEFAULT_LOGGER_BUF_SIZE, DEFAULT_LOGGER_QUEUE_BUF_SIZE, DEFAULT_FILE, DEFAULT_LOG_QUEUE};

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Logger,
    Profiler,
    None,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn to_bp3d_debug(&self) -> bp3d_debug::logger::Level {
        match self {
            Level::Trace => bp3d_debug::logger::Level::Trace,
            Level::Debug => bp3d_debug::logger::Level::Debug,
            Level::Info => bp3d_debug::logger::Level::Info,
            Level::Warning => bp3d_debug::logger::Level::Warn,
            Level::Error => bp3d_debug::logger::Level::Error,
        }
    }

    pub fn to_filter(&self) -> bp3d_logger::LevelFilter {
        match self {
            Level::Trace => bp3d_logger::LevelFilter::Trace,
            Level::Debug => bp3d_logger::LevelFilter::Debug,
            Level::Info => bp3d_logger::LevelFilter::Info,
            Level::Warning => bp3d_logger::LevelFilter::Warn,
            Level::Error => bp3d_logger::LevelFilter::Error,
        }
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Auto,
    Always,
    Never,
}

impl Color {
    pub fn to_logger(self) -> bp3d_logger::Colors {
        match self {
            Color::Auto => bp3d_logger::Colors::Auto,
            Color::Always => bp3d_logger::Colors::Enabled,
            Color::Never => bp3d_logger::Colors::Disabled
        }
    }
}

#[derive(Deserialize)]
pub struct Console {
    pub enabled: Option<bool>,
    pub color: Option<Color>,
    pub stderr: Option<bool>
}

impl Console {
    pub fn get_color(&self) -> Color {
        self.color.unwrap_or(DEFAULT_COLOR)
    }

    pub fn get_stderr(&self) -> bool {
        self.stderr.unwrap_or(DEFAULT_STDERR)
    }

    pub fn get_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

#[derive(Deserialize)]
pub struct File {
    pub enabled: Option<bool>
}

impl File {
    pub fn get_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LogQueue {
    pub enabled: Option<bool>,
    pub buf_size: Option<usize>
}

impl LogQueue {
    pub fn get_enabled(&self) -> bool {
        self.enabled.unwrap_or_default()
    }

    pub fn get_buf_size(&self) -> usize {
        self.buf_size.unwrap_or(DEFAULT_LOGGER_QUEUE_BUF_SIZE)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Local {
    pub level: Option<Level>,
    pub console: Option<Console>,
    pub file: Option<File>,
    pub queue: Option<LogQueue>,
    pub buf_size: Option<usize>
}

impl Local {
    pub fn get_level(&self) -> Level {
        self.level.unwrap_or(DEFAULT_LEVEL)
    }

    pub fn get_console(&self) -> &Console {
        self.console.as_ref().unwrap_or(&DEFAULT_CONSOLE)
    }

    pub fn get_buf_size(&self) -> usize {
        self.buf_size.unwrap_or(DEFAULT_LOGGER_BUF_SIZE)
    }

    pub fn get_file(&self) -> &File {
        self.file.as_ref().unwrap_or(&DEFAULT_FILE)
    }

    pub fn get_queue(&self) -> &LogQueue {
        self.queue.as_ref().unwrap_or(&DEFAULT_LOG_QUEUE)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Remote {
    pub port: Option<u16>,
    pub max_rows: Option<u32>,
    pub min_period: Option<u16>,
    pub buf_size: Option<usize>
}

impl Remote {
    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(DEFAULT_PORT)
    }

    pub fn get_max_rows(&self) -> u32 {
        self.max_rows.unwrap_or(DEFAULT_MAX_ROWS)
    }

    pub fn get_min_period(&self) -> u16 {
        self.min_period.unwrap_or(DEFAULT_MIN_PERIOD)
    }

    pub fn get_buf_size(&self) -> usize {
        self.buf_size.unwrap_or(DEFAULT_REMOTE_BUF_SIZE)
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub mode: Option<Mode>,
    pub local: Option<Local>,
    pub remote: Option<Remote>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: None,
            local: None,
            remote: None,
        }
    }
}

impl Config {
    pub fn get_mode(&self) -> Mode {
        self.mode.unwrap_or(DEFAULT_MODE)
    }

    pub fn get_local(&self) -> &Local {
        self.local.as_ref().unwrap_or(&DEFAULT_LOGGER)
    }

    pub fn get_remote(&self) -> &Remote {
        self.remote.as_ref().unwrap_or(&DEFAULT_PROFILER)
    }
}
