use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;
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

fn bench_task_spawner_state_operations(c: &mut Criterion) {
    c.bench_function("task_spawner_get_state", |b| {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("state_bench_task".to_string(), config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        b.iter(|| rt.block_on(async { black_box(spawner.get_state().await) }))
    });

    c.bench_function("task_spawner_get_task_info", |b| {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("info_bench_task".to_string(), config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        b.iter(|| rt.block_on(async { black_box(spawner.get_task_info().await) }))
    });

    c.bench_function("task_spawner_is_running", |b| {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("running_bench_task".to_string(), config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        b.iter(|| rt.block_on(async { black_box(spawner.is_running().await) }))
    });
}

fn bench_task_execution_lightweight(c: &mut Criterion) {
    // Benchmark lightweight task execution (no actual process spawning)
    c.bench_function("task_config_validation", |b| {
        let config = TaskConfig::new("echo")
            .args(vec!["hello".to_string(), "world".to_string()])
            .working_dir(
                std::env::current_dir()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            )
            .timeout_ms(5000);

        b.iter(|| black_box(config.validate().unwrap()))
    });

    c.bench_function("task_spawner_creation_with_validation", |b| {
        b.iter(|| {
            let config = TaskConfig::new("echo")
                .args(vec!["benchmark".to_string()])
                .timeout_ms(1000);

            // Validate config (this is what would happen in real usage)
            config.validate().unwrap();

            black_box(TaskSpawner::new("validated_bench_task".to_string(), config))
        })
    });
}

fn bench_concurrent_task_creation(c: &mut Criterion) {
    // Benchmark creating multiple tasks concurrently
    for task_count in [1, 5, 10, 20].iter() {
        c.bench_with_input(
            BenchmarkId::new("concurrent_task_creation", task_count),
            task_count,
            |b, &task_count| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.iter(|| {
                    rt.block_on(async move {
                        let mut handles = Vec::new();

                        for i in 0..task_count {
                            let handle = tokio::spawn(async move {
                                let config =
                                    TaskConfig::new("echo").args(vec![format!("task_{}", i)]);

                                let spawner =
                                    TaskSpawner::new(format!("concurrent_bench_{}", i), config);

                                black_box(spawner)
                            });

                            handles.push(handle);
                        }

                        // Wait for all tasks to be created
                        let spawners = futures::future::try_join_all(handles).await.unwrap();
                        black_box(spawners)
                    })
                })
            },
        );
    }
}

fn bench_memory_usage_patterns(c: &mut Criterion) {
    // Benchmark memory allocation patterns
    c.bench_function("task_config_clone", |b| {
        let config = TaskConfig::new("echo")
            .args(vec!["arg1".to_string(), "arg2".to_string()])
            .working_dir("/tmp".to_string())
            .timeout_ms(5000);

        b.iter(|| black_box(config.clone()))
    });

    c.bench_function("task_config_with_large_args", |b| {
        b.iter(|| {
            let large_args: Vec<String> = (0..100)
                .map(|i| format!("argument_{}_with_some_content", i))
                .collect();

            let config = TaskConfig::new("echo").args(large_args);
            black_box(config)
        })
    });

    c.bench_function("task_config_with_large_env", |b| {
        b.iter(|| {
            let large_env: std::collections::HashMap<String, String> = (0..50)
                .map(|i| (format!("VAR_{}", i), format!("value_{}_content", i)))
                .collect();

            let config = TaskConfig::new("echo").env(large_env);
            black_box(config)
        })
    });
}

// Note: This benchmark spawns real processes, so it's slower and more realistic
// It's disabled by default to keep benchmark runs fast
#[allow(dead_code)]
fn bench_actual_process_execution(c: &mut Criterion) {
    c.bench_function("actual_echo_execution", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let (tx, mut rx) = mpsc::channel(100);

                let config = if cfg!(windows) {
                    TaskConfig::new("cmd")
                        .args(vec!["/C".to_string(), "echo benchmark".to_string()])
                } else {
                    TaskConfig::new("echo").args(vec!["benchmark".to_string()])
                };

                let mut spawner = TaskSpawner::new("benchmark_process".to_string(), config);

                let result = spawner.start_direct(tx).await;
                assert!(result.is_ok());

                // Wait for completion
                while let Ok(event) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await
                {
                    if let Some(event) = event {
                        if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                black_box(())
            })
        })
    });
}

criterion_group!(
    task_performance_benches,
    bench_task_spawner_creation,
    bench_task_spawner_state_operations,
    bench_task_execution_lightweight,
    bench_concurrent_task_creation,
    bench_memory_usage_patterns,
    // bench_actual_process_execution, // Uncomment for full process execution benchmarks
);

criterion_main!(task_performance_benches);
