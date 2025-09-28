use crate::tasks::{config::TaskConfig, tokio::spawn::spawner::TaskSpawner};

#[tokio::test]
async fn process_id_initially_none() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    assert_eq!(spawner.get_process_id().await, None);
}

// TODO: Consider adding serde support for TaskInfo, not skipping Instant fields
#[cfg(feature = "serde")]
#[tokio::test]
async fn task_info_serde() {
    use serde_json;

    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    let info = spawner.get_task_info().await;

    // This should work even with Instant fields skipped
    let serialized = serde_json::to_string(&info).unwrap();
    println!("Serialized TaskInfo: {}", serialized);
    assert!(serialized.contains("serde_task"));
    assert!(serialized.contains("Pending"));
}
