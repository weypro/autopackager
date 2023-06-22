use anyhow::{anyhow, Result};
use glob::glob;
use regex::Regex;
use serde_yaml;
use std::fs;
use std::process::Command as SysCommand;

// 引入serde_yaml库
use serde_yaml::{from_str, Value};

mod packager_command;

// 从yaml字符串中解析出命令列表
fn parse_commands(yaml: &str) -> Vec<packager_command::Command> {
    // 使用serde_yaml库的from_str函数将yaml字符串转换为Value类型
    let value: Value = from_str(yaml).unwrap();
    // 从Value中获取command字段，它应该是一个数组
    let command_array = value["command"].as_sequence().unwrap();
    // 创建一个空的命令列表
    let mut commands = Vec::new();
    // 遍历command数组中的每个元素，它们应该是一个映射
    for command_map in command_array {
        // 使用match表达式来匹配映射中存在的字段，并创建相应的枚举类变体
        let command = match (
            command_map.get("copy"),
            command_map.get("replace"),
            command_map.get("run"),
        ) {
            (Some(copy_value), None, None) => {
                // 如果只有copy字段，就创建Copy变体
                let copy = packager_command::Copy {
                    source: copy_value["source"].as_str().unwrap().to_string(),
                    destination: copy_value["destination"].as_str().unwrap().to_string(),
                    gitignore_path: copy_value["gitignore_path"].as_str().unwrap().to_string(),
                    use_gitignore: copy_value["use_gitignore"].as_bool().unwrap(),
                };
                packager_command::Command::Copy(copy)
            }
            (None, Some(replace_value), None) => {
                // 如果只有replace字段，就创建Replace变体
                let replace = packager_command::Replace {
                    source: replace_value["source"].as_str().unwrap().to_string(),
                    regex: replace_value["regex"].as_str().unwrap().to_string(),
                    replacement: replace_value["replacement"].as_str().unwrap().to_string(),
                };
                packager_command::Command::Replace(replace)
            }
            (None, None, Some(run_value)) => {
                // 如果只有run字段，就创建Run变体
                let run = packager_command::Run {
                    command: run_value["command"].as_str().unwrap().to_string(),
                };
                packager_command::Command::Run(run)
            }
            _ => panic!("Invalid command format"), // 如果有其他情况，就抛出异常
        };
        // 将Command枚举类添加到命令列表中
        commands.push(command);
    }
    // 返回命令列表
    commands
}

// 定义一个函数来执行命令列表
fn execute_commands(commands: &[packager_command::Command]) -> Result<()> {
    // 遍历命令列表中的每个命令
    for command in commands {
        // 使用match表达式来匹配命令的类型，并解构出关联数据
        match command {
            packager_command::Command::Copy(copy) => packager_command::execute_copy(copy)?,
            packager_command::Command::Replace(replace) => {
                packager_command::execute_replace(replace)?
            }
            packager_command::Command::Run(run) => packager_command::execute_run(run)?,
        }
    }
    // 如果没有错误，就返回Ok(())
    Ok(())
}

fn parse_commands_from_yaml(file_path: &str) -> Result<Vec<packager_command::Command>> {
    // 从yaml文件中读取内容，并存储为一个字符串
    let mut yaml_content = fs::read_to_string(file_path)?;
    // 使用serde_yaml库的from_str函数将yaml字符串转换为Value类型
    let value: Value = from_str(&yaml_content).unwrap();
    // 从Value中获取define_items字段，它应该是一个数组
    if let Some(define_items) = value["define_items"].as_sequence() {
        // 遍历define_items数组中的每个元素，它们应该是一个映射
        for var_item in define_items {
            // 从映射中获取item字段，它应该是一个映射
            if let Some(item) = var_item["item"].as_mapping() {
                // 从item映射中获取key和value字段，它们应该是字符串
                if let (Some(key), Some(value)) = (
                    item.get(&Value::from("key")),
                    item.get(&Value::from("value")),
                ) {
                    if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                        // 使用format!函数将key和value拼接成"{key}"和value的形式
                        let key = format!("{{{}}}", key);
                        let value = value.to_string();
                        // 使用regex::Regex::new函数创建一个正则表达式对象，用来匹配"{key}"
                        let re = Regex::new(&regex::escape(&key)).unwrap();
                        // 使用regex::Regex::replace_all函数将文件内容中的"{key}"替换为value
                        yaml_content = re.replace_all(&yaml_content, &value).to_string();
                    }
                }
            }
        }
    }

    let commands = parse_commands(&yaml_content);
    Ok(commands)
}

// 定义一个主函数来测试程序
fn main() {
    // 将yaml解析为命令列表
    let commands = parse_commands_from_yaml("config.yaml").unwrap();
    println!("{:#?}", commands);

    // 调用execute_commands函数，执行命令列表
    execute_commands(&commands).unwrap();
}
