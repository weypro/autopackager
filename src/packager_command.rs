use anyhow::{anyhow, Result};
use glob::glob;
use regex::Regex;
use serde_yaml;
use std::fs;
use std::process::Command as SysCommand;

// 引入serde_yaml库
use serde_yaml::{from_str, Value};


// 定义一个枚举类来存储命令
#[derive(Debug)]
pub enum Command {
    Copy(Copy),       // copy命令的变体，关联一个Copy结构体
    Replace(Replace), // replace命令的变体，关联一个Replace结构体
    Run(Run),         // run命令的变体，关联一个Run结构体
}

// 定义一个结构体来存储copy命令的参数
#[derive(Debug)]
pub struct Copy {
    pub source: String,
    pub destination: String,
    pub gitignore_path: String,
    pub use_gitignore: bool,
}

// 定义一个结构体来存储replace命令的参数
#[derive(Debug)]
pub struct Replace {
    pub source: String,
    pub regex: String,
    pub replacement: String,
}

// 定义一个结构体来存储run命令的参数
#[derive(Debug)]
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
