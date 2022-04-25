![logo](https://assets.gitlab-static.net/uploads/-/system/group/avatar/10553166/logo_white.ico?width=64)

# Tracing

Implementations of `tracing` [Subscriber](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html) for use with
BP3D software. Supports traditional logging through bp3d-logger and supports remote profiling through TCP.

## Status

This crate is currently **highly experimental** and may not work on all systems. Currently, this is fully tested on
`macOS 12.3.1` only. It may work on other `unix` systems.

Additionally, this is currently broken on `Windows` systems.

## Usage

### In code

```rust
fn main() {
    let _guard = bp3d_tracing::initialize("my-application");
    //Application code using `tracing` or `log` utilities.
}
```

**NOTE:** this library uses threads to perform logging and profiling. As such you shouldn't call `std::process::exit()`
without first dropping the `_guard` guard variable. Failing to do so will result in truncated logs with the logger
backend and data loss with the profiler backend.

### Running

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
| linux   | ~/.local/share/my-application/Documents/environment                |
| Windows | %APPDATA%/my-application/Documents/environment                     |
