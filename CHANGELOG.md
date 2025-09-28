# Changelog

## 0.4

### 0.4.1 (2025/09/26)
#### Fixed
- In validator function, environment variable key should not contain tab and newline characters [[commit](https://github.com/xpcn2015/tcrm-task/commit/0140ab862fb7cb1735b202fe05ec60a3ac1f0b92)]
#### Changed
- Simplify TaskEvent enum by removing task_name fields [[commit](https://github.com/xpcn2015/tcrm-task/commit/b99a4c8fa9c388b982b4c7a0725562dffc47eff9)]
- Add 'os.system(' to injection patterns in validator [[commit](https://github.com/xpcn2015/tcrm-task/commit/1c5829039f878235019cd21a57955bba3e33df34)]
- Add '#' as invalid pattern in validate_command_strict() [[commit](https://github.com/xpcn2015/tcrm-task/commit/c45a2743605a83a16aa4069dcc413cb94e8beeea)]
- process group logic now guard behide "process-group" feature
- Single executor handles multiple async operations using tokio::select! (version 0.3 using tokio::spawn to handle multiple operations) [[commit](https://github.com/xpcn2015/tcrm-task/commit/ae5b9fc041e6ad119b95d3cf696fbcfb95e771ad)]

## 0.3

### 0.3.7 (2025/09/24)
#### Fixed
- Ready event is now correctly emitted only when the indicator string matches [[commit](https://github.com/xpcn2015/tcrm-task/commit/3bfc67ce5b9b9a7a2f0dd8da33e08c0c39066e98)]
- Update dependencies

### 0.3.6 (2025/09/18)
#### Added
- Optional process group management: `TaskConfig::use_process_group(bool)` allows enabling/disabling cross-platform process group/job object usage for child process tracking. Default is enabled. [[commit](https://github.com/xpcn2015/tcrm-task/commit/37a007cf13753979852206a7fd397d99f30e27ae)]
- Example `process_group_optional.rs` demonstrating both enabled and disabled modes, with platform-specific commands for Windows and Unix.

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

