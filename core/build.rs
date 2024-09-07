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

const WRITE_FAIL: &str = "Failed to write verision_inject file";

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use bp3d_protoc::gen::RustParams;
use bp3d_protoc::generate_rust;
use semver::Version;
use std::io::Write;

fn generate_version_inject() {
    let path = std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .expect("Couldn't obtain cargo OUT_DIR")
        .join("version_inject.rs");
    let file = File::create(path).expect("Couldn't create version_inject file");
    let mut writer = BufWriter::new(file);
    let version = std::env::var("CARGO_PKG_VERSION").expect("Unable to read package version");
    let version = Version::parse(&version).expect("Failed to parse package version");
    write!(&mut writer, "const VERSION_DATA: [u8; 24] = [").expect(WRITE_FAIL);
    let len = if !version.pre.is_empty() {
        let len = std::cmp::min(24, version.pre.as_bytes().len());
        for v in 0..len {
            write!(&mut writer, "0x{:X}, ", version.pre.as_bytes()[v]).expect(WRITE_FAIL);
        }
        len
    } else {
        0
    };
    for _ in 0..24 - len {
        write!(&mut writer, "0x0, ").expect(WRITE_FAIL);
    }
    writeln!(&mut writer, "];\n").expect(WRITE_FAIL);
    writeln!(&mut writer, "const MAJOR_VERSION: u64 = {};", version.major).expect(WRITE_FAIL);
}

fn main() {
    generate_rust(|loader| {
        loader.load("./src/profiler/network/hello.json5")?;
        loader.load("./src/profiler/network/message.json5")?;
        loader.load("./src/profiler/network/value.json5")
    }, |protoc| protoc, RustParams::default().enable_write_async(true));
    generate_rust(|loader| {
        loader.import("./src/profiler/network/value.json5", "crate::profiler::network::value")?;
        loader.load("./src/profiler/network/common.json5")
    }, |protoc| protoc.set_reads_messages(false), RustParams::default().enable_write_async(true));
    generate_rust(|loader| {
        loader.import("./src/profiler/network/value.json5", "crate::profiler::network::value")?;
        loader.import("./src/profiler/network/common.json5", "crate::profiler::network::common")?;
        loader.load("./src/profiler/network/profiler.json5")?;
        loader.load("./src/profiler/network/event.json5")?;
        loader.load("./src/profiler/network/span.json5")
    }, |protoc| protoc.set_reads_messages(false), RustParams::default().enable_write_async(true));
    generate_rust(|loader| {
        loader.import("./src/profiler/network/value.json5", "crate::profiler::network::value")?;
        loader.import("./src/profiler/network/common.json5", "crate::profiler::network::common")?;
        loader.import("./src/profiler/network/event.json5", "crate::profiler::network::event")?;
        loader.import("./src/profiler/network/profiler.json5", "crate::profiler::network::profiler")?;
        loader.load("./src/profiler/network/client.json5")?;
        loader.load("./src/profiler/network/server.json5")
    }, |protoc| protoc.set_writes_messages(false), RustParams::default());
    generate_version_inject();
}
