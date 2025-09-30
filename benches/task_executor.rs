use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use tcrm_task::tasks::{
    config::{StreamSource, TaskConfig},
    tokio::executor::TaskExecutor,
};
use tokio::sync::mpsc;

fn criterion_benchmark(c: &mut Criterion) {
    // Benchmark basic process spawning and completion
    c.bench_function("task_executor_basic_process", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            #[cfg(windows)]
            let config = TaskConfig::new("cmd").args(["/C", "echo", "benchmark_test"]);

            #[cfg(unix)]
            let config = TaskConfig::new("echo").args(["benchmark_test"]);

            let config = config.use_process_group(false);
            let mut executor = TaskExecutor::new(black_box(config));
            let (tx, mut rx) = mpsc::channel(100);

            executor.coordinate_start(tx).await.unwrap();

            // Consume all events until completion
            while let Some(event) = rx.recv().await {
                // Process event (black_box prevents optimization)
                let is_stopped =
                    matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. });
                black_box(event);
                if is_stopped {
                    break;
                }
            }
        });
    });

    // Benchmark process with multiple output lines
    c.bench_function("task_executor_multiple_output", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            #[cfg(windows)]
            let config = TaskConfig::new("powershell")
                .args(["-Command", "1..10 | ForEach-Object { Write-Output $_ }"]);

            #[cfg(unix)]
            let config =
                TaskConfig::new("sh").args(["-c", "for i in $(seq 1 10); do echo $i; done"]);

            let config = config.use_process_group(false);
            let mut executor = TaskExecutor::new(black_box(config));
            let (tx, mut rx) = mpsc::channel(100);

            executor.coordinate_start(tx).await.unwrap();

            let mut output_count = 0;
            while let Some(event) = rx.recv().await {
                match event {
                    tcrm_task::tasks::event::TaskEvent::Output { .. } => {
                        output_count += 1;
                    }
                    tcrm_task::tasks::event::TaskEvent::Stopped { .. } => break,
                    _ => {}
                }
                black_box(event);
            }
            black_box(output_count);
        });
    });

    // Benchmark ready indicator detection
    c.bench_function("task_executor_ready_indicator", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            #[cfg(windows)]
            let config = TaskConfig::new("cmd")
                .args(["/C", "echo", "READY_SIGNAL"])
                .ready_indicator("READY_SIGNAL")
                .ready_indicator_source(StreamSource::Stdout);

            #[cfg(unix)]
            let config = TaskConfig::new("echo")
                .args(["READY_SIGNAL"])
                .ready_indicator("READY_SIGNAL")
                .ready_indicator_source(StreamSource::Stdout);

            let config = config.use_process_group(false);
            let mut executor = TaskExecutor::new(black_box(config));
            let (tx, mut rx) = mpsc::channel(100);

            executor.coordinate_start(tx).await.unwrap();

            let mut ready_detected = false;
            while let Some(event) = rx.recv().await {
                match event {
                    tcrm_task::tasks::event::TaskEvent::Ready => {
                        ready_detected = true;
                    }
                    tcrm_task::tasks::event::TaskEvent::Stopped { .. } => break,
                    _ => {}
                }
                black_box(event);
            }
            black_box(ready_detected);
        });
    });

    // Benchmark configuration validation
    c.bench_function("task_config_validation", |b| {
        b.iter(|| {
            let config = TaskConfig::new("echo")
                .args(["test", "validation", "performance"])
                .timeout_ms(30000)
                .enable_stdin(true)
                .ready_indicator("ready")
                .ready_indicator_source(StreamSource::Stdout)
                .use_process_group(false);

            black_box(config.validate()).unwrap();
        });
    });

    // Benchmark TaskExecutor creation
    c.bench_function("task_executor_creation", |b| {
        b.iter(|| {
            let config = TaskConfig::new("echo").args(["creation_test"]);
            let executor = TaskExecutor::new(black_box(config));
            black_box(executor);
        });
    });

    // Benchmark concurrent process execution
    c.bench_function("task_executor_concurrent_processes", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let handles = (0..5)
                .map(|i| {
                    tokio::spawn(async move {
                        #[cfg(windows)]
                        let config = TaskConfig::new("cmd").args([
                            "/C",
                            "echo",
                            &format!("concurrent_{}", i),
                        ]);
                        #[cfg(unix)]
                        let config = TaskConfig::new("echo").args([format!("concurrent_{}", i)]);

                        let config = config.use_process_group(false);
                        let mut executor = TaskExecutor::new(config);
                        let (tx, mut rx) = mpsc::channel(100);

                        executor.coordinate_start(tx).await.unwrap();

                        while let Some(event) = rx.recv().await {
                            if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
                                break;
                            }
                        }
                    })
                })
                .collect::<Vec<_>>();

            for handle in handles {
                handle.await.unwrap();
            }
        });
    });

    // Benchmark with timeout (short-running to avoid actual timeout)
    c.bench_function("task_executor_with_timeout", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            #[cfg(windows)]
            let config = TaskConfig::new("cmd")
                .args(["/C", "echo", "timeout_test"])
                .timeout_ms(5000); // 5 second timeout, won't trigger for echo

            #[cfg(unix)]
            let config = TaskConfig::new("echo")
                .args(["timeout_test"])
                .timeout_ms(5000); // 5 second timeout, won't trigger for echo

            let config = config.use_process_group(false);
            let mut executor = TaskExecutor::new(black_box(config));
            let (tx, mut rx) = mpsc::channel(100);

            executor.coordinate_start(tx).await.unwrap();

            while let Some(event) = rx.recv().await {
                if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
                    break;
                }
                black_box(event);
            }
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(50);
    targets = criterion_benchmark
}
criterion_main!(benches);
