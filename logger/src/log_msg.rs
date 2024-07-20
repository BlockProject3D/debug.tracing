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

use std::fmt::{Error, Write};
use std::mem::MaybeUninit;
use bp3d_debug::logger::Level;
use bp3d_debug::util::Location;
use time::OffsetDateTime;

// Size of the control fields of the log message structure:
// 40 bytes of Location structure (&'static str is 16 bytes) + 16 bytes of OffsetDateTime + 4 bytes of msg len + 1 byte of Level + 3 bytes of padding
const LOG_CONTROL_SIZE: usize = 40 + 16 + 4 + 1 + 3;
// Limit the size of the log message string so that the size of the log structure is LOG_BUFFER_SIZE
const LOG_MSG_SIZE: usize = LOG_BUFFER_SIZE - LOG_CONTROL_SIZE;
const LOG_BUFFER_SIZE: usize = 1024;

/// A log message.
///
/// This structure uses a large 1K buffer which stores the entire log message to improve
/// performance.
///
/// The repr(C) is used to force the control fields (msg_len, level and target_len) to be before
/// the message buffer and avoid large movs when setting control fields.
///
/// # Examples
///
/// ```
/// use bp3d_logger::LogMsg;
/// use std::fmt::Write;
/// use bp3d_debug::logger::Level;
/// use bp3d_debug::util::Location;
/// let mut msg = LogMsg::new(Location::new("test", "file.c", 1), Level::Info);
/// let _ = write!(msg, "This is a formatted message {}", 42);
/// assert_eq!(msg.msg(), "This is a formatted message 42");
/// ```
#[derive(Clone)]
#[repr(C)]
pub struct LogMsg {
    location: Location,
    time: OffsetDateTime,
    msg_len: u32,
    level: Level,
    buffer: [MaybeUninit<u8>; LOG_MSG_SIZE],
}

impl LogMsg {
    /// Creates a new instance of log message buffer.
    ///
    /// # Arguments
    ///
    /// * `location`: the location this message comes from.
    /// * `level`: the [Level](Level) of the log message.
    ///
    /// returns: LogMsg
    ///
    /// # Examples
    ///
    /// ```
    /// use bp3d_debug::logger::Level;
    /// use bp3d_debug::util::Location;
    /// use bp3d_logger::LogMsg;
    /// let msg = LogMsg::new(Location::new("test", "file.c", 1), Level::Info);
    /// assert_eq!(msg.location().module_path(), "test");
    /// assert_eq!(msg.level(), Level::Info);
    /// ```
    pub fn new(location: Location, level: Level) -> LogMsg {
        LogMsg::with_time(location, OffsetDateTime::now_utc(), level)
    }

    /// Creates a new instance of log message buffer.
    ///
    /// # Arguments
    ///
    /// * `location`: the location this message comes from.
    /// * `level`: the [Level](Level) of the log message.
    ///
    /// returns: LogMsg
    ///
    /// # Examples
    ///
    /// ```
    /// use bp3d_debug::logger::Level;
    /// use bp3d_debug::util::Location;
    /// use time::macros::datetime;
    /// use bp3d_logger::LogMsg;
    /// let msg = LogMsg::with_time(Location::new("test", "file.c", 1), datetime!(1999-1-1 0:0 UTC), Level::Info);
    /// assert_eq!(msg.location().module_path(), "test");
    /// assert_eq!(msg.level(), Level::Info);
    /// ```
    pub fn with_time(location: Location, time: OffsetDateTime, level: Level) -> LogMsg {
        LogMsg {
            location,
            time,
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            msg_len: 0,
            level,
        }
    }

    /// Clears the log message but keep the target and the level.
    ///
    /// # Examples
    ///
    /// ```
    /// use bp3d_debug::logger::Level;
    /// use bp3d_debug::util::Location;
    /// use bp3d_logger::LogMsg;
    /// let mut msg = LogMsg::from_msg(Location::new("test", "file.c", 1), Level::Info, "this is a test");
    /// msg.clear();
    /// assert_eq!(msg.msg(), "");
    /// assert_eq!(msg.location().module_path(), "test");
    /// assert_eq!(msg.level(), Level::Info);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.msg_len = 0;
    }

    /// Replaces the time contained in this log message.
    ///
    /// # Arguments
    ///
    /// * `time`: the new [OffsetDateTime](OffsetDateTime).
    ///
    /// returns: ()
    pub fn set_time(&mut self, time: OffsetDateTime) {
        self.time = time;
    }

    /// Auto-creates a new log message with a pre-defined string message.
    ///
    /// This function is the same as calling [write](LogMsg::write) after [new](LogMsg::new).
    ///
    /// # Arguments
    ///
    /// * `target`: the target name this log comes from.
    /// * `level`: the [Level](Level) of the log message.
    /// * `msg`: the message string.
    ///
    /// returns: LogMsg
    ///
    /// # Examples
    ///
    /// ```
    /// use bp3d_debug::logger::Level;
    /// use bp3d_debug::util::Location;
    /// use bp3d_logger::LogMsg;
    /// let mut msg = LogMsg::from_msg(Location::new("test", "file.c", 1), Level::Info, "this is a test");
    /// assert_eq!(msg.location().module_path(), "test");
    /// assert_eq!(msg.level(), Level::Info);
    /// assert_eq!(msg.msg(), "this is a test");
    /// ```
    pub fn from_msg(location: Location, level: Level, msg: &str) -> LogMsg {
        let mut ads = Self::new(location, level);
        unsafe { ads.write(msg.as_bytes()) };
        ads
    }

    /// Appends a raw byte buffer at the end of the message buffer.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Arguments
    ///
    /// * `buf`: the raw byte buffer to append.
    ///
    /// returns: usize
    ///
    /// # Safety
    ///
    /// * [LogMsg](LogMsg) contains only valid UTF-8 strings so buf must contain only valid UTF-8
    /// bytes.
    /// * If buf contains invalid UTF-8 bytes, further operations on the log message buffer may
    /// result in UB.
    #[allow(clippy::missing_transmute_annotations)]
    pub unsafe fn write(&mut self, buf: &[u8]) -> usize {
        let len = std::cmp::min(buf.len(), LOG_MSG_SIZE - self.msg_len as usize);
        if len > 0 {
            std::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                std::mem::transmute(self.buffer.as_mut_ptr().offset(self.msg_len as _)),
                len,
            );
            self.msg_len += len as u32; //The length is always less than 2^32.
        }
        len
    }

    /// Returns the location the log message comes from.
    #[inline]
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// Returns the time of this log message.
    #[inline]
    pub fn time(&self) -> &OffsetDateTime {
        &self.time
    }

    /// Returns the log message as a string.
    #[inline]
    #[allow(clippy::missing_transmute_annotations)]
    pub fn msg(&self) -> &str {
        // SAFETY: This is always safe because LogMsg is always UTF-8.
        unsafe {
            std::str::from_utf8_unchecked(std::mem::transmute(&self.buffer[..self.msg_len as _]))
        }
    }

    /// Returns the level of this log message.
    #[inline]
    pub fn level(&self) -> Level {
        self.level
    }
}

impl Write for LogMsg {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        unsafe {
            self.write(s.as_bytes());
        }
        Ok(())
    }
}
