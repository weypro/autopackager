use ansi_term;
// use anyhow::{anyhow, Result};
use clap::Parser;
// use libc::{setlocale, LC_ALL};
// use std::ffi::CString;
use std::{env, path::Path};
use tracing::{error, info, trace, warn};
use tracing_subscriber;
mod packager_command;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // 配置文件路径
    #[arg(short, long)]
    config: String,
    #[arg(short, long)]
    workdir: Option<String>,
}

fn main() {
    // 为win10启用ansi颜色支持
    #[cfg(target_os = "windows")]
    {
        ansi_term::enable_ansi_support().unwrap_or({
            error!("color is not supported");
        });
    }

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    // let locale = CString::new("zh_CN.UTF-8").unwrap();
    // unsafe {
    //     setlocale(LC_ALL, locale.as_ptr());
    // }

    let args = Args::parse();

    info!("starting packager...");
    trace!("The config file path is: {}", args.config);

    let config_dir: &str = &args.config;
    let config = match packager_command::parse_commands_from_yaml(config_dir, true) {
        Ok(config) => {
            trace!("read file successfully");
            config
        }
        Err(e) => {
            error!(error = ?e, "failed to read file");
            return;
        }
    };

    // 打印Config对象的内容，验证反序列化是否正确
    println!("{:#?}", config);

    match args.workdir {
        None => {
            // 如果没有传入工作路径参数，则根据配置文件路径来设置当前工作路径
            if let Some(config_parent_dir) = Path::new(config_dir).parent() {
                if config_parent_dir.is_dir() {
                    if let Err(e) = env::set_current_dir(config_parent_dir) {
                        error!("Failed to change current directory: {}", e);
                    } else {
                        info!(
                            "Successfully changed current directory to {}",
                            config_parent_dir.display()
                        );
                    }
                } else {
                    warn!("Already in the directory");
                }
            } else {
                warn!("The path has no parent");
            }
        }
        Some(path) => {
            if let Err(e) = env::set_current_dir(&path) {
                error!("Failed to change current directory: {}", e);
            } else {
                info!("Successfully changed current directory to {}", path);
            }
        }
    }

    match packager_command::execute_commands(&config.command) {
        Ok(_) => {
            info!("All commands executed successfully!");
        }
        Err(e) => {
            error!(
                "{} error(s) occurred in {} command(s)!",
                e.len(),
                config.command.len()
            );
        }
    }
}
