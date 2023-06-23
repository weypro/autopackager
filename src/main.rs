use anyhow::{anyhow, Result};
use tracing::{error, info};
use tracing_subscriber;

mod packager_command;


fn main() {
    tracing_subscriber::fmt::init();
    info!("starting packager...");

    let config =
        match packager_command::parse_commands_from_yaml(&String::from("config.yaml"), true) {
            Ok(config) => {
                info!("read file successfully");
                config
            }
            Err(e) => {
                error!(error = ?e, "failed to read file");
                return;
            }
        };

    // 打印Config对象的内容，验证反序列化是否正确
    //println!("{:#?}", config);

    match packager_command::execute_commands(&config.command) {
        Ok(_) => {
            info!("All commands executed successfully!");
        }
        Err(e) => {
            error!("{} error(s) occurred!", e.len());
        }
    }
}
