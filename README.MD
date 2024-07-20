![logo](https://assets.gitlab-static.net/uploads/-/system/group/avatar/10553166/logo_white.ico?width=64)

# Debugger implementation

This repository contains the main implementation of bp3d-debug as well as all needed support tools.
This is the official implementation of bp3d-debug for use within all BP3D software.

## BP3D logger

A flexible log system intended to be used with BP3D software.

### Main features
- Asynchronous to avoid blocking any of the application threads.
- Multithreaded compatible.
- Local time based logging (configurable).
- Support for file logging.
- Support for stdout/stderr logging.
- Support for in memory logger.
- Easily switch on/off logging backends.
- Default logging backends should not panic/abort/crash the software in any way.

#### stdout/stderr
- Error messages are written to stderr instead of stdout (configurable).
- If write fails, message is ignored; no panic produced.

#### file logging
- If write fails, message is ignored.

#### in memory
- Log is limited to a fixed number of messages (configurable).

## BP3D Tracing

This crate contains the actual implementation for bp3d-debug.
Supports traditional logging through bp3d-logger and supports remote profiling through TCP.

### Status

This crate is currently **highly experimental** and may not work on all systems. Currently, this is fully tested on
`macOS 12.3.1` only. It may work on other `unix` systems.

Additionally, this is currently broken on `Windows` systems.

### Usage

#### In code

```rust
fn main() {
    let _guard = bp3d_tracing::initialize("my-application");
    //Application code using `tracing` or `log` utilities.
}
```

**NOTE:** this library uses threads to perform logging and profiling. As such you shouldn't call `std::process::exit()`
without first dropping the `_guard` guard variable. Failing to do so will result in truncated logs with the logger
backend and data loss with the profiler backend.

#### Running

To configure the behavior of `bp3d-tracing` some environment variables are used:

| Name        | Type    | Possible values                    | Behavior                              | Default |
|-------------|---------|------------------------------------|---------------------------------------|---------|
| PROFILER    | boolean | 1, 0, true, false, on, off         | Enables remote profiling.             | off     |
| LOG         | enum    | trace, debug, info, warning, error | Maximum log level for logger backend. | info    |
| LOG_DISABLE | boolean | 1, 0, true, false, on, off         | Disables the logger backend.          | off     |
| LOG_STDOUT  | boolean | 1, 0, true, false, on, off         | Always print log messages to stdout.  | off     |
| LOG_COLOR   | boolean | 1, 0, true, false, on, off         | Enables color output.                 | isatty  |

On systems where configuring environment variables is not expected or too complicated, this library supports `bp3d-env`.

`bp3d-env` supports loading files as fallback to environment variables. This library appends the following default path
to `bp3d-env` `bp3d-fs::App::new("my-application").get_documents()?.join("environment")` which corresponds to the
following paths:

| OS      | Path                                                               |
|---------|--------------------------------------------------------------------|
| macOS   | ~/Library/Application Support/my-application/Documents/environment |
| Linux   | ~/.local/share/my-application/Documents/environment                |
| Windows | %APPDATA%/my-application/Documents/environment                     |
| iOS     | Files App/my-application/environment                               |
