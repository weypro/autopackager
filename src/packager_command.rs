use anyhow::{anyhow, Result};
use glob::glob;
use regex::Regex;
use serde_yaml;
use std::fs;
use std::process::Command as SysCommand;

use serde::{Serialize, Deserialize};


// 定义一个结构体，表示整个yaml对象
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Config {
    pub define_items: Vec<DefineItem>,
    pub command: Vec<Command>,
}


// 定义一个结构体，表示定义项
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DefineItem {
    pub key: String,
    pub value: String,
}

// 定义一个枚举类来存储命令
#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "type")]
pub enum Command {
    Copy(Copy),       // copy命令的变体，关联一个Copy结构体
    Replace(Replace), // replace命令的变体，关联一个Replace结构体
    Run(Run),         // run命令的变体，关联一个Run结构体
}

// 定义一个结构体来存储copy命令的参数
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Copy {
    pub source: String,
    pub destination: String,
    pub gitignore_path: String,
    pub use_gitignore: bool,
}

// 定义一个结构体来存储replace命令的参数
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Replace {
    pub source: String,
    pub regex: String,
    pub replacement: String,
}

// 定义一个结构体来存储run命令的参数
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Run {
    pub command: String,
}


// 定义一个函数来执行copy命令
pub fn execute_copy(copy: &Copy) -> Result<()> {
    // 输出提示
    println!("Copying files from {} to {}", copy.source, copy.destination);
    println!("Using gitignore file at {}", copy.gitignore_path);
    println!("Using gitignore rules? {}", copy.use_gitignore);
    // 如果没有错误，就返回Ok(())
    Ok(())
}

// 定义一个函数来执行replace命令
pub fn execute_replace(replace: &Replace) -> Result<()> {
    // 输出提示
    println!(
        "Replacing {} with {} in {}",
        replace.regex, replace.replacement, replace.source
    );

    // 创建一个正则表达式对象
    let regex = Regex::new(&replace.regex)?;

    // 遍历匹配源路径的所有文件
    for entry in glob(&replace.source)? {
        match entry {
            Ok(path) => {
                if !path.is_file() {
                    continue;
                }
                // 读取文件内容并替换匹配的部分
                let file_content = fs::read_to_string(&path)?;
                let replaced_content = regex
                    .replace_all(&file_content, &replace.replacement)
                    .to_owned()
                    .to_string();
                // 写入新的文件内容
                fs::write(&path, replaced_content)?;
            }
            Err(e) => return Err(anyhow!("Failed to read glob pattern. {}", e.to_string())),
        }
    }

    Ok(())

    // Ok(())
}

// 定义一个函数来执行run命令
pub fn execute_run(run: &Run) -> Result<()> {
    // 输出提示
    println!("Running command: {}", run.command);

    // 根据操作系统选择不同的命令
    let output = if cfg!(target_os = "windows") {
        SysCommand::new("cmd")
            .args(["/C", &run.command])
            .output()
            .expect("failed to execute process")
    } else {
        SysCommand::new("sh")
            .arg("-c")
            .arg(&run.command)
            .output()
            .expect("failed to execute process")
    };

    // 检查命令是否成功
    if output.status.success() {
        // 输出标准输出和标准错误
        println!(
            "Running command result: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        //println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Ok(())
    } else {
        // 返回错误值
        Err(anyhow!("command failed with status: {}", output.status))
    }
}


// 定义一个函数来执行命令列表
pub fn execute_commands(commands: &[Command]) -> Result<()> {
    // 遍历命令列表中的每个命令
    for command in commands {
        // 使用match表达式来匹配命令的类型，并解构出关联数据
        match command {
            Command::Copy(copy) => execute_copy(copy)?,
            Command::Replace(replace) => {
                execute_replace(replace)?
            }
            Command::Run(run) => execute_run(run)?,
        }
    }
    // 如果没有错误，就返回Ok(())
    Ok(())
}

// 从yaml文件中反序列化Config
pub fn parse_commands_from_yaml(file_path: &str) -> Result<Config> {
    // 从yaml文件中读取内容，并存储为一个字符串
    let mut yaml_content = fs::read_to_string(file_path)?;
    // 使用serde_yaml库的from_str函数将yaml字符串转换为Value类型
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml_content).unwrap();
    // 从Value中获取define_items字段，它应该是一个数组
    if let Some(define_items) = value["define_items"].as_sequence() {
        // 遍历define_items数组中的每个元素，它们应该是一个映射
        for item in define_items {
            // 从映射中获取item字段，它应该是一个映射
            // 从item映射中获取key和value字段，它们应该是字符串
            if let (Some(key), Some(value)) = (
                item.get(&serde_yaml::Value::from("key")),
                item.get(&serde_yaml::Value::from("value")),
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

    deserialize_config(&yaml_content)
}


// 定义一个函数，用于从yaml字符串反序列化为Config对象
pub fn deserialize_config(yaml: &str) -> Result<Config> {
    Ok(serde_yaml::from_str(yaml).unwrap())
}

