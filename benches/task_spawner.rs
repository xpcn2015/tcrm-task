use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tcrm_task::tasks::{
    async_tokio::spawner::TaskSpawner,
    config::{StreamSource, TaskConfig},
};
use tokio::sync::mpsc;

fn bench_task_spawner_creation(c: &mut Criterion) {
    c.bench_function("task_spawner_new", |b| {
        b.iter(|| {
            let config = TaskConfig::new("echo");
            black_box(TaskSpawner::new("bench_task".to_string(), config))
        })
    });

    c.bench_function("task_spawner_with_complex_config", |b| {
        b.iter(|| {
            let mut config = TaskConfig::new("echo")
                .args(vec!["arg1".to_string(), "arg2".to_string()])
                .working_dir("/tmp".to_string())
                .timeout_ms(5000)
                .enable_stdin(true)
                .ready_indicator("READY".to_string())
                .ready_indicator_source(StreamSource::Stdout);
            config.env = Some(
                vec![
                    ("KEY1".to_string(), "value1".to_string()),
                    ("KEY2".to_string(), "value2".to_string()),
                ]
                .into_iter()
                .collect(),
            );
            black_box(TaskSpawner::new("complex_bench_task".to_string(), config))
        })
    });

    c.bench_function("task_spawner_uptime", |b| {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("uptime_bench_task".to_string(), config);
        b.iter(|| black_box(spawner.uptime()))
    });

    c.bench_function("task_spawner_with_stdin", |b| {
        b.iter(|| {
            let config = TaskConfig::new("cat").enable_stdin(true);
            let (_, rx) = mpsc::channel(100);
            black_box(TaskSpawner::new("stdin_bench_task".to_string(), config).set_stdin(rx))
        })
    });
}

fn bench_task_spawner_start_direct(c: &mut Criterion) {
    c.bench_function("task_spawner_start_direct", |b| {
        b.iter(|| {
            let config = if cfg!(target_os = "windows") {
                TaskConfig::new("Powershell").args(vec!["echo".to_string()])
            } else {
                TaskConfig::new("bash").args(vec!["echo".to_string()])
            };

            let (tx, _rx) = mpsc::channel(100);
            let mut spawner = TaskSpawner::new("start_direct_bench_task".to_string(), config);
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                black_box(spawner.start_direct(tx).await.unwrap());
            });
        })
    });
}

criterion_group!(
    task_performance_benches,
    bench_task_spawner_creation,
    bench_task_spawner_start_direct,
);

criterion_main!(task_performance_benches);
