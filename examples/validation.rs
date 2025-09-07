//! Example: Configuration validation
use tcrm_task::tasks::config::TaskConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TaskConfig::new("") // Invalid: empty command
        .timeout_ms(1000);
    match config.validate() {
        Ok(_) => println!("Config is valid"),
        Err(e) => println!("Config error: {}", e),
    }
    Ok(())
}
