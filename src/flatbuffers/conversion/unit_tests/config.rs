use std::collections::HashMap;

use crate::{
    flatbuffers::{
        conversion::{ConversionError, ToFlatbuffers},
        tcrm_task_generated,
    },
    tasks::config::{StreamSource, TaskConfig},
};

#[test]
fn stream_source_conversions() {
    // Test FlatBuffer to Rust conversion
    assert_eq!(
        StreamSource::try_from(tcrm_task_generated::tcrm::task::StreamSource::Stdout).unwrap(),
        StreamSource::Stdout
    );
    assert_eq!(
        StreamSource::try_from(tcrm_task_generated::tcrm::task::StreamSource::Stderr).unwrap(),
        StreamSource::Stderr
    );

    // Test Rust to FlatBuffer conversion
    assert_eq!(
        tcrm_task_generated::tcrm::task::StreamSource::from(StreamSource::Stdout),
        tcrm_task_generated::tcrm::task::StreamSource::Stdout
    );
    assert_eq!(
        tcrm_task_generated::tcrm::task::StreamSource::from(StreamSource::Stderr),
        tcrm_task_generated::tcrm::task::StreamSource::Stderr
    );
}
#[test]
fn config_roundtrip() {
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "value1".to_string());
    env.insert("KEY2".to_string(), "value2".to_string());
    let original_config = TaskConfig::new("test_command")
        .args(["arg1", "arg2"])
        .working_dir("/tmp")
        .env(env)
        .timeout_ms(5000)
        .enable_stdin(true)
        .ready_indicator("READY")
        .ready_indicator_source(StreamSource::Stderr)
        .use_process_group(false);

    // Convert to FlatBuffer
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = original_config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);

    // Get bytes and create new FlatBuffer instance
    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();

    // Convert back to Rust
    let converted_config = TaskConfig::try_from(fb_config).unwrap();

    // Verify roundtrip
    assert_eq!(original_config.command, converted_config.command);
    assert_eq!(original_config.args, converted_config.args);
    assert_eq!(original_config.working_dir, converted_config.working_dir);
    assert_eq!(original_config.env, converted_config.env);
    assert_eq!(original_config.timeout_ms, converted_config.timeout_ms);
    assert_eq!(original_config.enable_stdin, converted_config.enable_stdin);
    assert_eq!(
        original_config.ready_indicator,
        converted_config.ready_indicator
    );
    assert_eq!(
        original_config.ready_indicator_source,
        converted_config.ready_indicator_source
    );
}

#[test]
fn minimal() {
    let original_config = TaskConfig::new("minimal");

    // Convert to FlatBuffer and back
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = original_config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);

    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
    let converted_config = TaskConfig::try_from(fb_config).unwrap();

    assert_eq!(original_config.command, converted_config.command);
    assert_eq!(original_config.args, converted_config.args);
    assert_eq!(original_config.working_dir, converted_config.working_dir);
    assert_eq!(original_config.env, converted_config.env);
    assert_eq!(converted_config.timeout_ms, None); // 0 converts to None
    assert_eq!(converted_config.enable_stdin, Some(false)); // default false
    assert_eq!(
        original_config.ready_indicator,
        converted_config.ready_indicator
    );
    assert_eq!(
        converted_config.ready_indicator_source,
        Some(StreamSource::Stdout)
    ); // default
    assert_eq!(
        original_config.use_process_group,
        converted_config.use_process_group
    );
}

#[test]
fn empty_args() {
    let config = TaskConfig::new("test").args(vec![] as Vec<String>);

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);
    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
    let converted = TaskConfig::try_from(fb_config).unwrap();

    assert_eq!(converted.args, Some(vec![]));
}

#[test]
fn empty_env() {
    let config = TaskConfig::new("test").env(HashMap::<String, String>::new());

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);
    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
    let converted = TaskConfig::try_from(fb_config).unwrap();

    assert_eq!(converted.env, Some(HashMap::new()));
}

#[test]
fn large_timeout() {
    let config = TaskConfig::new("test").timeout_ms(u64::MAX);

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);
    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
    let converted = TaskConfig::try_from(fb_config).unwrap();

    assert_eq!(converted.timeout_ms, Some(u64::MAX));
}

#[test]
fn unicode_strings() {
    let config = TaskConfig::new("测试命令")
        .args(["参数1", "🚀", "ทดสอบ"])
        .working_dir("/tmp/测试")
        .ready_indicator("准备就绪(พร้อมทำงาน)");

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);
    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
    let converted = TaskConfig::try_from(fb_config).unwrap();

    assert_eq!(converted.command, "测试命令");
    let args = converted.args.unwrap();
    assert_eq!(args[0], "参数1");
    assert_eq!(args[1], "🚀");
    assert_eq!(args[2], "ทดสอบ");
    assert_eq!(converted.working_dir.unwrap(), "/tmp/测试");
    assert_eq!(converted.ready_indicator.unwrap(), "准备就绪(พร้อมทำงาน)");
}

#[test]
fn stream_source_invalid() {
    let invalid_source = tcrm_task_generated::tcrm::task::StreamSource(99);
    let result = StreamSource::try_from(invalid_source);
    assert!(result.is_err());

    if let Err(ConversionError::InvalidStreamSource(val)) = result {
        assert_eq!(val, 99);
    } else {
        panic!("Expected InvalidStreamSource error");
    }
}
#[test]
fn stress_test() {
    // Create a config with many environment variables
    let mut env = HashMap::new();
    for i in 0..100 {
        env.insert(format!("KEY_{}", i), format!("value_{}", i));
    }

    let mut args = Vec::new();
    for i in 0..50 {
        args.push(format!("arg_{}", i));
    }

    let config = TaskConfig {
        command: "stress_test_command".to_string(),
        args: Some(args),
        working_dir: Some("/tmp/stress_test".to_string()),
        env: Some(env.clone()),
        timeout_ms: Some(30000),
        enable_stdin: Some(true),
        ready_indicator: Some("STRESS_READY".to_string()),
        ready_indicator_source: Some(StreamSource::Stderr),
        use_process_group: Some(true),
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);
    let bytes = builder.finished_data();
    let fb_config =
        flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
    let converted = TaskConfig::try_from(fb_config).unwrap();

    assert_eq!(converted.env.unwrap().len(), 100);
    assert_eq!(converted.args.unwrap().len(), 50);
    assert_eq!(converted.command, "stress_test_command");
}
