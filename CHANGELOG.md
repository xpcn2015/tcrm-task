# Changelog

## 0.2.1 (2025/09/08)
### Fixed
- Correctly set `process_id` to `None` after task is stopped
- Ensure `process_id` is `Some` while task is running
- Added tests for process_id lifecycle and task state

## 0.2.0 (2025/09/08)
### Changed
- Changed `TaskEvent::Error` to use `TaskError` instead of string
- Modified `TaskError::IO` to use `String` instead of `std::io::Error`
- Enhanced error propagation: task spawner related error now properly emit `TaskEvent::Error` with structured `TaskError` before failing

## 0.1.0 (2025/09/07) [YANKED]
### Added
- Initial release
- Asynchronous task execution with Tokio
- Task configuration and validation
- Event system for task lifecycle and output
- Timeout and termination support
- Example programs: basic, interactive stdin, validation
- FlatBuffers serialization support (optional)
- Optional tracing/logging feature (enable with `tracing` Cargo feature)

