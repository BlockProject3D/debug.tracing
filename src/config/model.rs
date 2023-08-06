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

use serde::Deserialize;

use super::defaults::{DEFAULT_LEVEL, DEFAULT_COLOR, DEFAULT_STDERR, DEFAULT_TIME_FORMAT, DEFAULT_PORT, DEFAULT_MAX_ROWS, DEFAULT_MODE, DEFAULT_LOGGER, DEFAULT_PROFILER, DEFAULT_MIN_PERIOD};

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Logger,
    Profiler,
    None
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error
}

impl Level {
    pub fn to_tracing(&self) -> tracing_core::Level {
        match self {
            Level::Trace => tracing_core::Level::TRACE,
            Level::Debug => tracing_core::Level::DEBUG,
            Level::Info => tracing_core::Level::INFO,
            Level::Warning => tracing_core::Level::WARN,
            Level::Error => tracing_core::Level::ERROR,
        }
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Auto,
    Always,
    Never
}

#[derive(Deserialize)]
pub struct Console {
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
}

#[derive(Deserialize)]
pub struct File {
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Logger {
    pub level: Option<Level>,
    pub time_format: Option<String>,
    pub console: Option<Console>,
    pub file: Option<File>
}

impl Logger {
    pub fn get_level(&self) -> Level {
        self.level.unwrap_or(DEFAULT_LEVEL)
    }

    pub fn get_time_format(&self) -> &str {
        self.time_format.as_deref().unwrap_or(DEFAULT_TIME_FORMAT)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Profiler {
    pub port: Option<u16>,
    pub max_rows: Option<u32>,
    pub min_period: Option<u16>
}

impl Profiler {
    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(DEFAULT_PORT)
    }

    pub fn get_max_rows(&self) -> u32 {
        self.max_rows.unwrap_or(DEFAULT_MAX_ROWS)
    }

    pub fn get_min_period(&self) -> u16 {
        self.min_period.unwrap_or(DEFAULT_MIN_PERIOD)
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub mode: Option<Mode>,
    pub logger: Option<Logger>,
    pub profiler: Option<Profiler>
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: None,
            logger: None,
            profiler: None
        }
    }
}

impl Config {
    pub fn get_mode(&self) -> Mode {
        self.mode.unwrap_or(DEFAULT_MODE)
    }

    pub fn get_logger(&self) -> &Logger {
        self.logger.as_ref().unwrap_or(&DEFAULT_LOGGER)
    }

    pub fn get_profiler(&self) -> &Profiler {
        self.profiler.as_ref().unwrap_or(&DEFAULT_PROFILER)
    }
}
