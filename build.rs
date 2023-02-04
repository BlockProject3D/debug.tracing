use semver::Version;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

const WRITE_FAIL: &str = "Failed to write verision_inject file";

fn main() {
    let path = std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .expect("Couldn't obtain cargo OUT_DIR")
        .join("version_inject.rs");
    let file = File::create(path).expect("Couldn't create version_inject file");
    let mut writer = BufWriter::new(file);
    let version = std::env::var("CARGO_PKG_VERSION").expect("Unable to read package version");
    let version = Version::parse(&version).expect("Failed to parse package version");
    if !version.pre.is_empty() {
        write!(&mut writer, "const VERSION_DATA: [u8; 24] = [").expect(WRITE_FAIL);
        let len = std::cmp::min(24, version.pre.as_bytes().len());
        for v in 0..len {
            write!(&mut writer, "0x{:X}, ", version.pre.as_bytes()[v]).expect(WRITE_FAIL);
        }
        for _ in 0..24 - len {
            write!(&mut writer, "0x0, ").expect(WRITE_FAIL);
        }
        writeln!(&mut writer, "];").expect(WRITE_FAIL);
        writeln!(
            &mut writer,
            "pub const HELLO_PACKET: Hello = Hello::new({}, Some(VERSION_DATA));",
            version.major
        )
        .expect(WRITE_FAIL);
    } else {
        writeln!(
            &mut writer,
            "pub const HELLO_PACKET: Hello = Hello::new({}, None);",
            version.major
        )
        .expect(WRITE_FAIL);
    }
}
