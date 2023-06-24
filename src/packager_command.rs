use anyhow::{anyhow, Result};
use glob::glob;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command as SysCommand;

use ignore::WalkBuilder;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_yaml;
use tracing::{error, info, trace};

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
    info!(
        "*** Copying files from {} to {}",
        copy.source, copy.destination
    );
    trace!("- Using gitignore file at {}", copy.gitignore_path);
    trace!("- Using gitignore rules? {}", copy.use_gitignore);

    if !Path::new(&copy.source).exists() {
        return Err(anyhow!("No such source directory"));
    }

    // 创建一个WalkBuilder迭代器，遍历源路径下的所有文件和目录
    let walker = if copy.use_gitignore {
        // 如果copy.use_gitignore为true，则添加ignore文件
        WalkBuilder::new(&copy.source)
            .add_custom_ignore_filename(&copy.gitignore_path)
            .clone()
    } else {
        // 否则，不添加ignore文件
        WalkBuilder::new(&copy.source)
            .git_ignore(copy.use_gitignore.clone())
            .ignore(copy.use_gitignore.clone())
            .git_global(copy.use_gitignore.clone())
            .clone()
    };

    // 创建一个WalkBuilder迭代器，遍历源路径下的所有文件和目录，并添加ignore文件
    for result in walker.build() {
        // 处理每个结果，如果是Ok(entry)，则获取entry的路径
        if let Ok(entry) = result {
            let entry_path = entry.path();

            // 判断entry是否是文件，如果是，则复制文件到目标路径
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                // 拼接目标路径和entry的相对路径，作为复制的目标路径
                let relative_path = entry_path.strip_prefix(&copy.source).unwrap();
                let target_path_str = format!("{}/{}", &copy.destination, relative_path.display());
                let target_path = Path::new(&target_path_str);

                // 创建文件的父目录，如果不存在的话
                if let Some(parent_path) = target_path.parent() {
                    fs::create_dir_all(parent_path)?;
                }
                // 复制文件到目标路径
                fs::copy(entry_path, &target_path)?;
            }
        } else {
            return Err(anyhow!("ERROR: {:?}", result));
        }
    }

    Ok(())
}

// 定义一个函数来执行replace命令
pub fn execute_replace(replace: &Replace) -> Result<()> {
    // 输出提示
    info!(
        "*** Replacing \"{}\" with \"{}\" in {}",
        replace.regex, replace.replacement, replace.source
    );

    // 创建一个正则表达式对象
    let regex = Regex::new(&replace.regex)?;

    // 验证源路径glob是否有效
    let path_list = glob(&replace.source)?;

    // 把迭代器转换成Vec
    let path_vec: Vec<_> = path_list.collect();
    // 输出长度
    // println!("路径列表的长度是: {}", path_vec.len());

    if path_vec.len() == 0 {
        return Err(anyhow!("Invalid path. No files found."));
    }

    // 遍历匹配源路径的所有文件
    for entry in path_vec {
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
            Err(e) => {
                return Err(anyhow!("Failed to read glob pattern. {}", e.to_string()));
            }
        }
    }

    Ok(())
}

// 定义一个函数来执行run命令
pub fn execute_run(run: &Run) -> Result<()> {
    // 输出提示
    info!("*** Running command: {}", run.command);

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
        info!("- result: {}", String::from_utf8_lossy(&output.stdout));
        //println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Ok(())
    } else {
        // 返回错误值
        Err(anyhow!("command failed with status: {}", output.status))
    }
}

// 定义一个函数来执行命令列表
pub fn execute_commands(commands: &[Command]) -> Result<(), Vec<anyhow::Error>> {
    // 使用partition_map方法来将Result分割成两个集合
    let (_, errors): (Vec<_>, Vec<_>) = commands
        .into_iter()
        .map(|command| match command {
            Command::Copy(copy) => execute_copy(copy),
            Command::Replace(replace) => execute_replace(replace),
            Command::Run(run) => execute_run(run),
        })
        // .partition_map(From::from);
        .partition_map(|r| match r {
            Ok(v) => itertools::Either::Left(v),
            Err(v) => {
                error!("!!! Error occurred: {}", v);
                itertools::Either::Right(v)
            }
        });
    // 检查错误集合是否为空
    if errors.is_empty() {
        // 如果没有错误，就返回Ok(())
        Ok(())
    } else {
        // 如果有错误，就返回Err(errors)
        Err(errors)
    }
}

// 从yaml文件中反序列化Config
pub fn parse_commands_from_yaml(file_path: &str, if_use_define: bool) -> Result<Config> {
    // 从yaml文件中读取内容，并存储为一个字符串
    let mut yaml_content = fs::read_to_string(file_path)?;

    if if_use_define {
        // 使用serde_yaml库的from_str函数将yaml字符串转换为Value类型
        // let value: serde_yaml::Value = serde_yaml::from_str(&yaml_content).unwrap();
        // // 从Value中获取define_items字段，它应该是一个数组
        // if let Some(define_items) = value["define_items"].as_sequence() {
        //     // 遍历define_items数组中的每个元素，它们应该是一个映射
        //     for item in define_items {
        //         // 从映射中获取item字段，它应该是一个映射
        //         // 从item映射中获取key和value字段，它们应该是字符串
        //         if let (Some(key), Some(value)) = (
        //             item.get(&serde_yaml::Value::from("key")),
        //             item.get(&serde_yaml::Value::from("value")),
        //         ) {
        //             if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
        //                 // 使用format!函数将key和value拼接成"{key}"和value的形式
        //                 let key = format!("{{{}}}", key);
        //                 let value = value.to_string();
        //                 // 使用regex::Regex::new函数创建一个正则表达式对象，用来匹配"{key}"
        //                 let re = Regex::new(&regex::escape(&key)).unwrap();
        //                 // 使用regex::Regex::replace_all函数将文件内容中的"{key}"替换为value
        //                 yaml_content = re.replace_all(&yaml_content, &value).to_string();
        //             }
        //         }
        //     }
        // }

        // 反序列化为Config结构体
        let config: Config = deserialize_config(&yaml_content)?;

        let mut old_config_str = yaml_content.clone();
        let mut new_config_str = old_config_str.clone();

        loop {
            for item in &config.define_items {
                // let re: Regex = Regex::new(&regex::escape(&format!("{{{}}}", item.key)))?;
                let key = format!("{{{}}}", item.key);
                let value = item.value.to_string();
                // 使用regex::Regex::new函数创建一个正则表达式对象，用来匹配"{key}"
                let re = Regex::new(&regex::escape(&key)).unwrap();
                new_config_str = re.replace_all(&new_config_str, &value).to_string();
            }

            if !old_config_str.eq(&new_config_str) {
                old_config_str = new_config_str.clone();
            } else {
                break;
            }
        }
        yaml_content = new_config_str.clone();
    }
    deserialize_config(&yaml_content)
}

// 定义一个函数，用于从yaml字符串反序列化为Config对象
pub fn deserialize_config(yaml: &str) -> Result<Config> {
    Ok(serde_yaml::from_str(yaml).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // 测试yaml文件解析
    fn parse_correct_commands_test() -> Result<()> {
        // 从tests/config.yml文件中解析出Config对象
        let config = parse_commands_from_yaml("tests/ori_data/config.yml", true)?;
        let expected_config = parse_commands_from_yaml("tests/data/config.yml", false)?;
        // 使用assert_eq!宏来断言两个Config对象是否相等
        assert_eq!(config, expected_config);
        // 如果没有错误，就返回Ok(())
        Ok(())
    }

    #[test]
    // 测试命令执行函数
    fn test_execute_run() {
        // 创建一个Run结构体实例
        let run = Run {
            command: "echo hello".to_string(),
        };

        // 调用execute_run函数，并断言它返回Ok(())
        assert_eq!((), execute_run(&run).unwrap());
    }
}
