use std::time::Duration;

use tokio::time::sleep;
use tokio::{sync::mpsc, time::timeout};

use crate::tasks::config::TaskConfig;
use crate::tasks::event::TaskStopReason;
use crate::tasks::process::action::stop::stop_process;
use crate::tasks::process::control::{ProcessControl, ProcessControlAction};
use crate::tasks::{
    config::StreamSource,
    control::TaskStatusInfo,
    event::TaskEvent,
    process::action::{pause::pause_process, resume::resume_process},
    tokio::{
        coordinate::integration_tests::helper::{
            expected_completed_executor_state, expected_started_executor_state,
        },
        executor::TaskExecutor,
    },
};

#[tokio::test]
async fn stop() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    // Create a long-running process to terminate
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "Start-Sleep 30"]);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep").args(["30"]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();

    let mut started = false;
    let mut stopped = false;
    let mut handle = None;

    // Wait for the process to start
    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id: pid,
                created_at,
                running_at,
            } => {
                started = true;
                expected_started_executor_state(&executor);
                assert_eq!(pid, executor.get_process_id().unwrap());
                assert_eq!(created_at, executor.get_create_at());
                assert_eq!(running_at, executor.get_running_at().unwrap());

                // Terminate the process after it starts
                handle = Some(tokio::task::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    stop_process(pid)
                }));
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                #[cfg(unix)]
                signal,
            } => {
                stopped = true;
                expected_completed_executor_state(&executor);
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);

                #[cfg(unix)]
                {
                    assert_eq!(exit_code, Some(1));
                    assert_eq!(signal, Some(SIGTERM));
                }
                #[cfg(windows)]
                {
                    assert_eq!(exit_code, Some(1));
                }
            }
            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Output { .. } | TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
            TaskEvent::ProcessControl { action } => {
                panic!("Unexpected ProcessControl action: {:?}", action);
            }
        }
    }

    assert!(started);
    assert!(stopped);
    handle.unwrap().await.unwrap().unwrap();
}

#[tokio::test]
async fn stop_from_executor() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    // Create a long-running process to terminate
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "Start-Sleep 30"]);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep").args(["30"]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();

    let mut started = false;
    let mut stopped = false;
    let mut process_stop_event = false;

    // Wait for the process to start
    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id: pid,
                created_at,
                running_at,
            } => {
                started = true;
                expected_started_executor_state(&executor);
                assert_eq!(pid, executor.get_process_id().unwrap());
                assert_eq!(created_at, executor.get_create_at());
                assert_eq!(running_at, executor.get_running_at().unwrap());

                // Terminate the process after it starts
                executor.stop_process().await.unwrap();
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                #[cfg(unix)]
                signal,
            } => {
                stopped = true;
                expected_completed_executor_state(&executor);
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);

                #[cfg(unix)]
                {
                    assert_eq!(exit_code, Some(1));
                    assert_eq!(signal, Some(SIGTERM));
                }
                #[cfg(windows)]
                {
                    assert_eq!(exit_code, Some(1));
                }
            }
            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Output { .. } | TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
            TaskEvent::ProcessControl { action } => match action {
                ProcessControlAction::Stop => {
                    process_stop_event = true;
                }
                _ => {
                    panic!("Unexpected ProcessControl action: {:?}", action);
                }
            },
        }
    }

    assert!(started);
    assert!(process_stop_event);
    assert!(stopped);
}

#[tokio::test]
async fn pause_resume() {
    use std::time::Instant;
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    // Create a process that outputs periodically
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args([
        "-Command",
        "for($i=1; $i -le 3; $i++) { Write-Host \"count: $i\"; Start-Sleep 1 }",
    ]);
    #[cfg(unix)]
    let config = TaskConfig::new("bash").args([
        "-c",
        "for i in {1..3}; do echo \"count: $i\"; sleep 1; done",
    ]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();

    let mut started = false;
    let mut process_id = None;
    let mut output_count = 0;
    let mut pause_tested = false;
    let mut _resume_tested = false;
    let mut stopped = false;
    let mut handle = None;

    let start_time = Instant::now();

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id: pid,
                created_at,
                running_at,
            } => {
                started = true;
                process_id = Some(pid);
                expected_started_executor_state(&executor);
                assert_eq!(pid, executor.get_process_id().unwrap());
                assert_eq!(created_at, executor.get_create_at());
                assert_eq!(running_at, executor.get_running_at().unwrap());
            }
            TaskEvent::Output { line, src } => {
                output_count += 1;
                assert_eq!(src, StreamSource::Stdout);
                assert!(line.contains("count:"));

                // After 2 outputs, pause the process, resume after 1 second
                if output_count == 2 && !pause_tested {
                    if let Some(pid) = process_id {
                        pause_tested = true;
                        handle = Some(tokio::task::spawn(async move {
                            pause_process(pid).unwrap();

                            // Wait 1 seconds then resume
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            resume_process(pid).unwrap();
                        }));
                    }
                }

                // After pause and resume, we should get more output
                if output_count > 2 && pause_tested {
                    _resume_tested = true;
                }
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                #[cfg(unix)]
                signal,
            } => {
                expected_completed_executor_state(&executor);
                assert_eq!(exit_code, Some(0));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                stopped = true;
                #[cfg(unix)]
                assert_eq!(signal, None);
            }
            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
            TaskEvent::ProcessControl { action } => {
                panic!("Unexpected ProcessControl action: {:?}", action);
            }
        }
    }

    let elapsed = start_time.elapsed();
    assert!(started);
    assert!(process_id.is_some());
    assert!(output_count >= 2);
    assert!(stopped);
    assert!(
        elapsed.as_secs_f32() >= 3.8 && elapsed.as_secs_f32() <= 4.4,
        "Elapsed time not between 3.8 and 4.4 seconds: {:?}",
        elapsed
    );
    handle.unwrap().await.unwrap();
}

#[tokio::test]
async fn pause_resume_from_executor() {
    use std::time::Instant;
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    // Create a process that outputs periodically
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args([
        "-Command",
        "for($i=1; $i -le 3; $i++) { Write-Host \"count: $i\"; Start-Sleep 1 }",
    ]);
    #[cfg(unix)]
    let config = TaskConfig::new("bash").args([
        "-c",
        "for i in {1..3}; do echo \"count: $i\"; sleep 1; done",
    ]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();

    let mut started = false;
    let mut process_id = None;
    let mut output_count = 0;
    let mut paused = false;
    let mut resumed = false;
    let mut stopped = false;

    let start_time = Instant::now();
    executor.pause_process().await.unwrap();
    sleep(Duration::from_secs(1)).await;
    executor.resume_process().await.unwrap();
    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id: pid,
                created_at,
                running_at,
            } => {
                started = true;
                process_id = Some(pid);
                expected_started_executor_state(&executor);
                assert_eq!(pid, executor.get_process_id().unwrap());
                assert_eq!(created_at, executor.get_create_at());
                assert_eq!(running_at, executor.get_running_at().unwrap());
            }
            TaskEvent::Output { line, src } => {
                output_count += 1;
                assert_eq!(src, StreamSource::Stdout);
                assert!(line.contains("count:"));
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                #[cfg(unix)]
                signal,
            } => {
                expected_completed_executor_state(&executor);
                assert_eq!(exit_code, Some(0));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                stopped = true;
                #[cfg(unix)]
                assert_eq!(signal, None);
            }
            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
            TaskEvent::ProcessControl { action } => match action {
                ProcessControlAction::Pause => {
                    paused = true;
                }
                ProcessControlAction::Resume => {
                    resumed = true;
                }
                ProcessControlAction::Stop => {
                    panic!("Unexpected Stop action");
                }
            },
        }
    }

    let elapsed = start_time.elapsed();
    assert!(started);
    assert!(process_id.is_some());
    assert!(output_count >= 2);
    assert!(paused);
    assert!(resumed);
    assert!(stopped);
    assert!(
        elapsed.as_secs_f32() >= 3.8 && elapsed.as_secs_f32() <= 4.4,
        "Elapsed time not between 3.8 and 4.4 seconds: {:?}",
        elapsed
    );
}
