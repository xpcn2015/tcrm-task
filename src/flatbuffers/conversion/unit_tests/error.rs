use crate::{
    flatbuffers::{
        conversion::{ConversionError, FromFlatbuffers, ToFlatbuffers},
        tcrm_task_generated,
    },
    tasks::error::TaskError,
};

#[test]
fn roundtrip() {
    let test_cases = vec![
        TaskError::IO("io error message".to_string()),
        TaskError::Handle("handle error message".to_string()),
        TaskError::Channel("channel error message".to_string()),
        TaskError::InvalidConfiguration("invalid config message".to_string()),
    ];

    for original_error in test_cases {
        // Convert to FlatBuffer
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_error = original_error.to_flatbuffers(&mut builder);
        builder.finish(fb_error, None);

        // Get bytes and create new FlatBuffer instance
        let bytes = builder.finished_data();
        let fb_error =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();

        // Convert back to Rust
        let converted_error = TaskError::from_flatbuffers(fb_error).unwrap();

        // Verify roundtrip
        match (&original_error, &converted_error) {
            (TaskError::IO(orig), TaskError::IO(conv)) => assert_eq!(orig, conv),
            (TaskError::Handle(orig), TaskError::Handle(conv)) => assert_eq!(orig, conv),
            (TaskError::Channel(orig), TaskError::Channel(conv)) => assert_eq!(orig, conv),
            (TaskError::InvalidConfiguration(orig), TaskError::InvalidConfiguration(conv)) => {
                assert_eq!(orig, conv)
            }
            _ => panic!(
                "Error type mismatch: {:?} vs {:?}",
                original_error, converted_error
            ),
        }
    }
}

#[test]
fn direct_read() {
    let error = TaskError::Channel("direct_channel_error".to_string());
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_error = error.to_flatbuffers(&mut builder);
    builder.finish(fb_error, None);
    let bytes = builder.finished_data();
    let fb = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();
    assert_eq!(
        fb.kind(),
        tcrm_task_generated::tcrm::task::TaskErrorType::Channel
    );
    assert_eq!(fb.message().unwrap(), "direct_channel_error");
}

#[test]
fn direct_read_all_error_types() {
    let errors = vec![
        (
            TaskError::IO("io test".to_string()),
            tcrm_task_generated::tcrm::task::TaskErrorType::IO,
        ),
        (
            TaskError::Handle("handle test".to_string()),
            tcrm_task_generated::tcrm::task::TaskErrorType::Handle,
        ),
        (
            TaskError::Channel("channel test".to_string()),
            tcrm_task_generated::tcrm::task::TaskErrorType::Channel,
        ),
        (
            TaskError::InvalidConfiguration("config test".to_string()),
            tcrm_task_generated::tcrm::task::TaskErrorType::InvalidConfiguration,
        ),
    ];

    for (error, expected_type) in errors {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_error = error.to_flatbuffers(&mut builder);
        builder.finish(fb_error, None);
        let bytes = builder.finished_data();
        let fb = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();
        assert_eq!(fb.kind(), expected_type);
        assert!(fb.message().unwrap().contains("test"));
    }
}
#[test]
fn unicode_message() {
    let error = TaskError::IO("Unicode error: 测试错误 🚀".to_string());
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_error = error.to_flatbuffers(&mut builder);
    builder.finish(fb_error, None);
    let bytes = builder.finished_data();
    let fb = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();

    assert_eq!(fb.message().unwrap(), "Unicode error: 测试错误 🚀");

    let converted = TaskError::from_flatbuffers(fb).unwrap();
    if let TaskError::IO(msg) = converted {
        assert_eq!(msg, "Unicode error: 测试错误 🚀");
    } else {
        panic!("Expected IO error");
    }
}

#[test]
fn empty_message() {
    let error = TaskError::Channel("".to_string());
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_error = error.to_flatbuffers(&mut builder);
    builder.finish(fb_error, None);
    let bytes = builder.finished_data();
    let fb = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();

    assert_eq!(fb.message().unwrap(), "");

    let converted = TaskError::from_flatbuffers(fb).unwrap();
    if let TaskError::Channel(msg) = converted {
        assert_eq!(msg, "");
    } else {
        panic!("Expected Channel error");
    }
}

#[test]
fn long_message() {
    let long_msg = "a".repeat(10000);
    let error = TaskError::IO(long_msg.clone());
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_error = error.to_flatbuffers(&mut builder);
    builder.finish(fb_error, None);
    let bytes = builder.finished_data();
    let fb = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();

    assert_eq!(fb.message().unwrap(), long_msg);

    let converted = TaskError::from_flatbuffers(fb).unwrap();
    if let TaskError::IO(msg) = converted {
        assert_eq!(msg, long_msg);
    } else {
        panic!("Expected IO error");
    }
}

#[test]
fn conversion_error_invalid_error_type() {
    // Test the error display for invalid error types
    let error = ConversionError::InvalidTaskErrorType(99);
    assert_eq!(error.to_string(), "Invalid TaskErrorType value: 99");
}
#[test]
fn conversion_error_display() {
    let errors = vec![
        ConversionError::InvalidStreamSource(99),
        ConversionError::InvalidTaskShell(88),
        ConversionError::InvalidTaskState(77),
        ConversionError::InvalidTaskTerminateReasonType(66),
        ConversionError::InvalidTaskEventStopReasonType(55),
        ConversionError::InvalidTaskEventType(44),
        ConversionError::InvalidTaskErrorType(33),
        ConversionError::MissingRequiredField("test_field"),
    ];

    for error in errors {
        let display_str = format!("{}", error);
        assert!(!display_str.is_empty());
        assert!(display_str.contains("Invalid") || display_str.contains("Missing"));
    }
}
