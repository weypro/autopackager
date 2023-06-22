use anyhow::{anyhow, Result};
use glob::glob;
use regex::Regex;
use serde_yaml;
use std::{fs};
use std::process::Command as SysCommand;

// 引入serde_yaml库
use serde_yaml::{from_str, Value};

// 定义一个枚举类来存储命令
#[derive(Debug)]
enum Command {
    Copy(Copy),       // copy命令的变体，关联一个Copy结构体
    Replace(Replace), // replace命令的变体，关联一个Replace结构体
    Run(Run),         // run命令的变体，关联一个Run结构体
}

// 定义一个结构体来存储copy命令的参数
#[derive(Debug)]
struct Copy {
    source: String,
    destination: String,
    gitignore_path: String,
    use_gitignore: bool,
}

// 定义一个结构体来存储replace命令的参数
#[derive(Debug)]
struct Replace {
    source: String,
    regex: String,
    replacement: String,
}

// 定义一个结构体来存储run命令的参数
#[derive(Debug)]
struct Run {
    command: String,
}

// 从yaml字符串中解析出命令列表
fn parse_commands(yaml: &str) -> Vec<Command> {
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
                let copy = Copy {
                    source: copy_value["source"].as_str().unwrap().to_string(),
                    destination: copy_value["destination"].as_str().unwrap().to_string(),
                    gitignore_path: copy_value["gitignore_path"].as_str().unwrap().to_string(),
                    use_gitignore: copy_value["use_gitignore"].as_bool().unwrap(),
                };
                Command::Copy(copy)
            }
            (None, Some(replace_value), None) => {
                // 如果只有replace字段，就创建Replace变体
                let replace = Replace {
                    source: replace_value["source"].as_str().unwrap().to_string(),
                    regex: replace_value["regex"].as_str().unwrap().to_string(),
                    replacement: replace_value["replacement"].as_str().unwrap().to_string(),
                };
                Command::Replace(replace)
            }
            (None, None, Some(run_value)) => {
                // 如果只有run字段，就创建Run变体
                let run = Run {
                    command: run_value["command"].as_str().unwrap().to_string(),
                };
                Command::Run(run)
            }
            _ => panic!("Invalid command format"), // 如果有其他情况，就抛出异常
        };
        // 将Command枚举类添加到命令列表中
        commands.push(command);
    }
    // 返回命令列表
    commands
}

// 定义一个函数来执行copy命令
fn execute_copy(copy: &Copy) -> Result<()> {
    // 输出提示
    println!("Copying files from {} to {}", copy.source, copy.destination);
    println!("Using gitignore file at {}", copy.gitignore_path);
    println!("Using gitignore rules? {}", copy.use_gitignore);
    // 如果没有错误，就返回Ok(())
    Ok(())
}

// 定义一个函数来执行replace命令
fn execute_replace(replace: &Replace) -> Result<()> {
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
               let replaced_content = regex.replace_all(&file_content, &replace.replacement).to_owned().to_string();
               // 写入新的文件内容
               fs::write(&path, replaced_content)?;
           }
           Err(e) => return Err(anyhow!("Failed to read glob pattern. {}",e.to_string())),
       }
   }

   Ok(())

    // Ok(())
}

// 定义一个函数来执行run命令
fn execute_run(run: &Run) -> Result<()> {
    // 输出提示
    println!("Running command: {}", run.command);
    
    // 根据操作系统选择不同的命令
    let output = if cfg!(target_os = "windows") {
        SysCommand::new("cmd")
                .args(["/C",&run.command])
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
        println!("Running command result: {}", String::from_utf8_lossy(&output.stdout));
        //println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Ok(())
    } else {
        // 返回错误值
        Err(anyhow!("command failed with status: {}", output.status))
    }
}


// 定义一个函数来执行命令列表
fn execute_commands(commands: &[Command]) -> Result<()> {
    // 遍历命令列表中的每个命令
    for command in commands {
        // 使用match表达式来匹配命令的类型，并解构出关联数据
        match command {
            Command::Copy(copy) => execute_copy(copy)?,
            Command::Replace(replace) => execute_replace(replace)?,
            Command::Run(run) => execute_run(run)?,
        }
    }
    // 如果没有错误，就返回Ok(())
    Ok(())
}

fn parse_commands_from_yaml(file_path: &str) -> Result<Vec<Command>> {
    let yaml_content = fs::read_to_string(file_path)?;
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
