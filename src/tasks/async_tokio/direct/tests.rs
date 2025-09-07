use tokio::sync::mpsc;
use tracing::Subscriber;
use tracing_subscriber::{
    Layer, Registry, fmt,
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
};

use crate::tasks::{
    async_tokio::spawner::TaskSpawner,
    config::{StreamSource, TaskConfig, TaskShell},
    error::TaskError,
    event::{TaskEvent, TaskEventStopReason},
    state::TaskTerminateReason,
};

struct TaskFilterLayer;

impl<S> Layer<S> for TaskFilterLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event, _ctx: Context<S>) {
        let mut visitor = TaskNameVisitor { found: false };
        event.record(&mut visitor);
        if visitor.found {
            println!("{:?}", event);
        }
    }
}

struct TaskNameVisitor {
    found: bool,
}

impl tracing::field::Visit for TaskNameVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "task_name" && value == "stdin_task" {
            self.found = true;
        }
    }
    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

#[tokio::test]
async fn start_direct_fn_echo_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let config = TaskConfig::new("echo")
        .args(["hello"])
        .shell(TaskShell::Auto);
    let mut spawner = TaskSpawner::new("echo_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started { task_name } => {
                assert_eq!(task_name, "echo_task");
                started = true;
            }
            TaskEvent::Output {
                task_name,
                line,
                src,
            } => {
                assert_eq!(task_name, "echo_task");
                assert_eq!(line, "hello");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                task_name,
                exit_code,
                reason: _,
            } => {
                assert_eq!(task_name, "echo_task");
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);
}
#[tokio::test]
async fn start_direct_fn_invalid_empty_command() {
    let (tx, _rx) = mpsc::channel::<TaskEvent>(1024);
    let config = TaskConfig::new(""); // invalid: empty command
    let mut spawner = TaskSpawner::new("bad_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));
}
#[tokio::test]
async fn start_direct_timeout_terminated_task() {
    let config = TaskConfig::new("sleep")
        .args(vec!["2"])
        .timeout_ms(500)
        .shell(TaskShell::Auto);
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let mut spawner = TaskSpawner::new("sleep_with_timeout_task".into(), config);

    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started { task_name } => {
                assert_eq!(task_name, "sleep_with_timeout_task");
                started = true;
            }

            TaskEvent::Stopped {
                task_name,
                exit_code,
                reason,
            } => {
                assert_eq!(task_name, "sleep_with_timeout_task");
                assert_eq!(exit_code, None);
                assert_eq!(
                    reason,
                    TaskEventStopReason::Terminated(TaskTerminateReason::Timeout)
                );
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);
}
#[tokio::test]
async fn start_direct_fn_stdin() {
    let subscriber = Registry::default()
        .with(TaskFilterLayer)
        .with(fmt::layer().with_target(false));

    tracing::subscriber::set_global_default(subscriber).unwrap();
    // Channel for receiving task events
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(1024);
    // Configure a task that reads a single line from stdin and echoes it
    #[cfg(windows)]
    let config = TaskConfig::new("$line = Read-Host; Write-Output $line")
        .shell(TaskShell::Powershell)
        // .timeout_ms(2000)
        .enable_stdin(true);

    #[cfg(unix)]
    let config = TaskConfig::new("read line; echo $line")
        .shell(TaskShell::Bash)
        .enable_stdin(true);

    let mut spawner = TaskSpawner::new("stdin_task".to_string(), config).set_stdin(stdin_rx);

    // Spawn the task
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    // Send input via stdin if enabled
    stdin_tx.send("hello world".to_string()).await.unwrap();

    let mut started = false;
    let mut output_ok = false;
    let mut stopped = false;

    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started { task_name } => {
                assert_eq!(task_name, "stdin_task");
                started = true;
            }
            TaskEvent::Output {
                task_name,
                line,
                src,
            } => {
                assert_eq!(task_name, "stdin_task");
                assert_eq!(line, "hello world");
                assert_eq!(src, StreamSource::Stdout);
                output_ok = true;
            }
            TaskEvent::Stopped {
                task_name,
                exit_code,
                ..
            } => {
                assert_eq!(task_name, "stdin_task");
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(output_ok);
    assert!(stopped);
}
