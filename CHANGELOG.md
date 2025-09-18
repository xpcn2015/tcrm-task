# Changelog

## 0.3

### 0.3.5 (2025/09/17)
#### Added
- Added `UserRequested` to `TaskTerminateReason` [[commit](https://github.com/xpcn2015/tcrm-task/commit/894d1ea0d30aafa90d8a737ff75307dd52bc6987)]


### 0.3.4 (2025/09/17)
#### Added
- Flatbuffers conversion traits
#### Changed
- Remove `Custom(String)` from `TaskTerminateReason` 
- Remove `Custom()` from `TaskError`
#### Fixed
- Documents typo
  
### 0.3.3 (2025/09/15)
#### Added
- Implemented FlatBuffers conversion.
#### Fixed
- Correct handling of optional exit codes in FlatBuffers using `-1` as sentinel value.
#### Changed
- Added `PartialEq` to `TaskError` and `TaskEvent`.

### 0.3.2 (2025/09/14)
#### Fixed
- Update `serde` dependencies
- Add Documents
- Fixed redundant pattern matching using `.is_err()` instead of `if let Err(_)`
- Refactored `spawn_std_watcher` function to use structured `OutputWatcherConfig` parameter instead of 8 individual parameters
  
### 0.3.1 (2025/09/10)
#### Fixed
- `TaskConfig::ready_indicator` setter now accepts `impl Into<String>` for flexibility.
- Task state is now set to `Ready` when the ready indicator is detected in the configured output stream.
- Updated unit test to verify state transition to `Ready` alongside `TaskEvent::Ready` emission.
#### Added
- Add trace log on task watchers for improved debugging.
#### Changed
- Rename FlatBuffers methods to match the official names.
- Make serde optional
  
### 0.3.0 (2025/09/09)
#### Added
- Introduced `TaskConfig::ready_indicator` and `TaskConfig::ready_indicator_source` fields to support readiness detection from process output (stdout or stderr).
- Integration and unit tests for ready indicator and source logic, ensuring `TaskEvent::Ready` is only emitted when the indicator appears in the configured stream.
#### Changed
- Removed `TaskSpawner::update_state_to_ready`; task state transitions are now managed exclusively by the task internals.

## 0.2

### 0.2.3 (2025/09/09)
#### Fixed
- Ensure `TaskState` is set to `Finished` if an error occurs during `start_direct` (configuration, process spawn, or process id failure)
- Added test to verify `TaskState` does not stall at `Initiating` after error in `start_direct`

### 0.2.2 (2025/09/09)
#### Added
- New `update_state_to_ready` method for `TaskSpawner` to set state to `Ready`
- Added unit test for `update_state_to_ready` method

### 0.2.1 (2025/09/08)
#### Fixed
- Correctly set `process_id` to `None` after task is stopped
- Ensure `process_id` is `Some` while task is running
- Added tests for process_id lifecycle and task state

### 0.2.0 (2025/09/08)
#### Changed
- Changed `TaskEvent::Error` to use `TaskError` instead of string
- Modified `TaskError::IO` to use `String` instead of `std::io::Error`
- Enhanced error propagation: task spawner related error now properly emit `TaskEvent::Error` with structured `TaskError` before failing

## 0.1.0 (2025/09/07) [YANKED]
#### Added
- Initial release
- Asynchronous task execution with Tokio
- Task configuration and validation
- Event system for task lifecycle and output
- Timeout and termination support
- Example programs: basic, interactive stdin, validation
- FlatBuffers serialization support (optional)
- Optional tracing/logging feature (enable with `tracing` Cargo feature)

