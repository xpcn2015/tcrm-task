use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tcrm_task::tasks::config::{StreamSource, TaskConfig};

fn bench_task_config_creations(c: &mut Criterion) {
    c.bench_function("task_config_new", |b| {
        b.iter(|| black_box(TaskConfig::new("echo")))
    });

    c.bench_function("task_config_builder_simple", |b| {
        b.iter(|| {
            black_box(
                TaskConfig::new("echo")
                    .args(vec!["hello".to_string()])
                    .working_dir("/tmp".to_string()),
            )
        })
    });

    c.bench_function("task_config_builder_complex", |b| {
        b.iter(|| {
            black_box(
                TaskConfig::new("echo")
                    .args(vec![
                        "arg1".to_string(),
                        "arg2".to_string(),
                        "arg3".to_string(),
                    ])
                    .working_dir("/tmp/complex".to_string())
                    .timeout_ms(10000)
                    .enable_stdin(true)
                    .ready_indicator("SERVER_READY".to_string())
                    .ready_indicator_source(StreamSource::Stderr),
            )
        })
    });

    c.bench_function("task_config_validation", |b| {
        b.iter(|| {
            let config = TaskConfig::new("echo")
                .ready_indicator("READY".to_string())
                .ready_indicator_source(StreamSource::Stdout);
            black_box(config.validate())
        })
    });

    c.bench_function("task_config_validation_invalid", |b| {
        b.iter(|| {
            let config = TaskConfig::new("").ready_indicator("".to_string());
            black_box(config.validate())
        })
    });
}

criterion_group!(task_performance_benches, bench_task_config_creations);

criterion_main!(task_performance_benches);
