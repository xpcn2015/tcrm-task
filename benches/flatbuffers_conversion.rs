use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::hint::black_box;
use tcrm_task::tasks::{
    config::{StreamSource, TaskConfig},
    error::TaskError,
    event::{TaskEvent, TaskEventStopReason},
    state::{TaskState, TaskTerminateReason},
};

#[cfg(feature = "flatbuffers")]
use tcrm_task::flatbuffers::tcrm_task_generated;

#[cfg(feature = "flatbuffers")]
fn bench_config_serialization(c: &mut Criterion) {
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "value1".to_string());
    env.insert("KEY2".to_string(), "value2".to_string());
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());

    let config = TaskConfig {
        command: "benchmark_command".to_string(),
        args: Some(vec![
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
        ]),
        working_dir: Some("/tmp/benchmark".to_string()),
        env: Some(env),
        timeout_ms: Some(5000),
        enable_stdin: Some(true),
        ready_indicator: Some("READY".to_string()),
        ready_indicator_source: Some(StreamSource::Stdout),
    };

    c.bench_function("config_to_flatbuffers", |b| {
        b.iter(|| {
            let mut builder = flatbuffers::FlatBufferBuilder::new();
            let fb_config = black_box(&config).to_flatbuffers(&mut builder);
            builder.finish(fb_config, None);
            black_box(builder.finished_data().to_vec())
        })
    });

    // Benchmark deserialization
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_config = config.to_flatbuffers(&mut builder);
    builder.finish(fb_config, None);
    let bytes = builder.finished_data();

    c.bench_function("config_from_flatbuffers", |b| {
        b.iter(|| {
            let fb_config =
                flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(black_box(bytes))
                    .unwrap();
            black_box(TaskConfig::try_from(fb_config).unwrap())
        })
    });
}

#[cfg(feature = "flatbuffers")]
fn bench_error_conversion(c: &mut Criterion) {
    let errors = vec![
        TaskError::IO("IO error benchmark".to_string()),
        TaskError::Handle("Handle error benchmark".to_string()),
        TaskError::Channel("Channel error benchmark".to_string()),
        TaskError::InvalidConfiguration("Invalid config benchmark".to_string()),
        TaskError::Custom("Custom error benchmark".to_string()),
    ];

    c.bench_function("error_to_flatbuffers", |b| {
        b.iter(|| {
            for error in &errors {
                let mut builder = flatbuffers::FlatBufferBuilder::new();
                let fb_error = black_box(error).to_flatbuffers(&mut builder);
                builder.finish(fb_error, None);
                black_box(builder.finished_data().to_vec());
            }
        })
    });
}

#[cfg(feature = "flatbuffers")]
fn bench_event_serialization(c: &mut Criterion) {
    let events = vec![
        TaskEvent::Started {
            task_name: "benchmark_task".to_string(),
        },
        TaskEvent::Output {
            task_name: "benchmark_task".to_string(),
            line: "This is a benchmark output line".to_string(),
            src: StreamSource::Stdout,
        },
        TaskEvent::Ready {
            task_name: "benchmark_task".to_string(),
        },
        TaskEvent::Stopped {
            task_name: "benchmark_task".to_string(),
            exit_code: Some(0),
            reason: TaskEventStopReason::Finished,
        },
        TaskEvent::Error {
            task_name: "benchmark_task".to_string(),
            error: TaskError::Custom("Benchmark error".to_string()),
        },
    ];

    c.bench_function("event_to_flatbuffers", |b| {
        b.iter(|| {
            for event in &events {
                let mut builder = flatbuffers::FlatBufferBuilder::new();
                let (_, offset) = black_box(event).to_flatbuffers(&mut builder);
                builder.finish(offset, None);
                black_box(builder.finished_data().to_vec());
            }
        })
    });
}

#[cfg(feature = "flatbuffers")]
fn bench_state_conversion(c: &mut Criterion) {
    let states = vec![
        TaskState::Pending,
        TaskState::Initiating,
        TaskState::Running,
        TaskState::Ready,
        TaskState::Finished,
    ];

    c.bench_function("state_conversion", |b| {
        b.iter(|| {
            for state in &states {
                let fb_state: tcrm_task_generated::tcrm::task::TaskState =
                    black_box(state).clone().into();
                let converted_back: TaskState = fb_state.try_into().unwrap();
                black_box(converted_back);
            }
        })
    });

    let terminate_reasons = vec![
        TaskTerminateReason::Timeout,
        TaskTerminateReason::Cleanup,
        TaskTerminateReason::DependenciesFinished,
        TaskTerminateReason::Custom("Benchmark custom reason".to_string()),
    ];

    c.bench_function("terminate_reason_to_flatbuffers", |b| {
        b.iter(|| {
            for reason in &terminate_reasons {
                let mut builder = flatbuffers::FlatBufferBuilder::new();
                let (_, offset) = black_box(reason).to_flatbuffers(&mut builder);
                builder.finish(offset, None);
                black_box(builder.finished_data().to_vec());
            }
        })
    });
}

#[cfg(feature = "flatbuffers")]
criterion_group!(
    flatbuffers_benches,
    bench_config_serialization,
    bench_error_conversion,
    bench_event_serialization,
    bench_state_conversion
);

#[cfg(not(feature = "flatbuffers"))]
criterion_group!(flatbuffers_benches,);

criterion_main!(flatbuffers_benches);
