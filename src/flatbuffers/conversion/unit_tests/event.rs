use crate::{
    flatbuffers::conversion::{FromFlatbuffers, ToFlatbuffers},
    tasks::{
        config::StreamSource,
        error::TaskError,
        event::{TaskEvent, TaskStopReason},
    },
};
use std::time::SystemTime;

/// Test TaskEvent::Started roundtrip conversion
#[test]
fn event_started_roundtrip() {
    let event = TaskEvent::Started {
        process_id: 12345,
        created_at: SystemTime::now(),
        running_at: SystemTime::now(),
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();

    // Compare individual fields since SystemTime may have nanosecond precision differences
    if let (
        TaskEvent::Started {
            process_id: pid1, ..
        },
        TaskEvent::Started {
            process_id: pid2, ..
        },
    ) = (&event, &roundtripped)
    {
        assert_eq!(pid1, pid2, "Process IDs should match");
    } else {
        panic!("Events should both be Started variants");
    }
}

/// Test TaskEvent::Output roundtrip conversion
#[test]
fn event_output_roundtrip() {
    let event = TaskEvent::Output {
        line: "Hello, World!".to_string(),
        src: StreamSource::Stdout,
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();
    assert_eq!(event, roundtripped);
}

/// Test TaskEvent::Ready roundtrip conversion
#[test]
fn event_ready_roundtrip() {
    let event = TaskEvent::Ready;

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();
    assert_eq!(event, roundtripped);
}

/// Test TaskEvent::Stopped roundtrip conversion
#[test]
fn event_stopped_roundtrip() {
    let event = TaskEvent::Stopped {
        exit_code: Some(0),
        reason: TaskStopReason::Finished,
        finished_at: SystemTime::now(),
        #[cfg(unix)]
        signal: None,
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();

    // Compare fields individually due to SystemTime precision
    if let (
        TaskEvent::Stopped {
            exit_code: ec1,
            reason: r1,
            ..
        },
        TaskEvent::Stopped {
            exit_code: ec2,
            reason: r2,
            ..
        },
    ) = (&event, &roundtripped)
    {
        assert_eq!(ec1, ec2, "Exit codes should match");
        assert_eq!(r1, r2, "Stop reasons should match");
    } else {
        panic!("Events should both be Stopped variants");
    }
}

/// Test TaskEvent::Error roundtrip conversion
#[test]
fn event_error_roundtrip() {
    let event = TaskEvent::Error {
        error: TaskError::IO("Test IO error".to_string()),
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();
    assert_eq!(event, roundtripped);
}

/// Test TaskEvent::ProcessControl roundtrip conversion
#[cfg(feature = "process-control")]
#[test]
fn event_process_control_roundtrip() {
    use crate::tasks::process::control::ProcessControlAction;

    let event = TaskEvent::ProcessControl {
        action: ProcessControlAction::Pause,
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();
    assert_eq!(event, roundtripped);
}
