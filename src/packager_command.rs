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
    //let newReg=replace.regex.replace("\\", "\\\\");
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

    let command = &run.command;
    let words = shell_words::split(command).unwrap(); // 用shell-words库来分割字符串
    let output = SysCommand::new(&words[0]) // words[0]是命令
        .args(&words[1..]) // words[1..]是参数组
        .output() // 执行命令并获取输出
        .or_else(|_| {
            // 如果失败了，就用cmd /c来执行
            if cfg!(target_os = "windows") {
                SysCommand::new("cmd").arg("/C").args(words).output()
            } else {
                SysCommand::new("sh").arg("-c").args(words).output()
            }
        })
        .expect("failed to execute command");

    // 检查命令是否成功
    if output.status.success() {
        // 输出标准输出和标准错误
        info!("- result: {}", String::from_utf8_lossy(&output.stdout));
        //println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Ok(())
    } else {
        error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
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
        // 反序列化为Config结构体
        let config: Config = deserialize_config(&yaml_content)?;

        // 建立变量名到值的映射关系
        let mut valuemap = std::collections::HashMap::new();
        for item in &config.define_items {
            valuemap.insert(item.key.clone(), item.value.clone());
        }

        // 对valuemap每一项进行遍历，进行变量替换
        for item in &config.define_items {
            let subst_value = substitute_variables(&item.value, &valuemap);
            valuemap.insert(item.key.clone(), subst_value);
        }

        // 对指定文本进行变量替换
        // let mut subst_text = yaml_content.clone();
        // for item in &config.define_items {
        //     subst_text = substitute_variables(&subst_text, &valuemap);
        // }

        let subst_text = substitute_variables(&yaml_content, &valuemap);
        println!("{}", subst_text);
        yaml_content = subst_text;

        // let mut old_config_str = yaml_content.clone();
        // let mut new_config_str = old_config_str.clone();

        // 循环替换，直到没有改变为止
        // loop {
        //     for item in &config.define_items {
        //         let key = format!("${{{}}}", item.key);
        //         let value = &item.value;
        //         // 使用regex::Regex::new函数创建一个正则表达式对象，用来匹配"{key}"
        //         let re = Regex::new(&regex::escape(&key)).unwrap();
        //         new_config_str = re.replace_all(&new_config_str, value).to_string();
        //     }

        //     if !old_config_str.eq(&new_config_str) {
        //         old_config_str = new_config_str.clone();
        //     } else {
        //         break;
        //     }
        // }

        // for item in &config.define_items {
        //     let key = format!("${{{}}}", item.key);
        //     let value = &item.value;
        //     // 使用regex::Regex::new函数创建一个正则表达式对象，用来匹配"{key}"
        //     let re = Regex::new(&regex::escape(&key)).unwrap();
        //     new_config_str = re.replace_all(&new_config_str, value).to_string();
        // }

        // let re = regex::Regex::new(r"$\{(\w+)\}").unwrap();
        // // 如果匹配到常量还存在，说明常量未完全定义
        // if re.is_match(&new_config_str) {
        //     return Err(anyhow!("Invalid configuration"));
        // }
        // yaml_content = new_config_str;
    }
    println!("{}", yaml_content);
    deserialize_config(&yaml_content)
}

// 替换字符串中的变量
// value 是待替换的字符串。
// valuemap 是一个 HashMap，用于将变量名映射到变量的值。
// 使用正则来查找 ${} 形式的变量名，并将变量名替换为变量的值。
// 在替换变量名时，函数会递归地调用自己来解析变量的值。这是因为变量的值可能包含其他变量名，例如 ${VER_MAJOR}.${VER_MINOR}.${VER_PATCH}.${VER_BUILD}。
// 在这种情况下，函数会首先替换 ${VER_MAJOR}，然后替换 ${VER_MINOR}，以此类推，直到所有变量都被替换为其对应的值。
// 请注意，函数会尝试替换所有变量，直到没有新的替换可以进行为止。如果某个变量的值中包含无法解析的变量名，函数将停止替换并返回原始字符串。这种情况可以在循环中检查，如果找不到相应的变量，则可以中断循环。
// 最后，该函数返回替换后的字符串。
fn substitute_variables(
    value: &str,
    valuemap: &std::collections::HashMap<String, String>,
) -> String {
    let re = Regex::new(r"\$\{(\w+)\}").unwrap();
    let mut result = String::from(value);
    while let Some(caps) = re.captures(&result) {
        let var_name = caps.get(1).unwrap().as_str();
        if let Some(subst_value) = valuemap.get(var_name) {
            let subst_result = substitute_variables(subst_value, valuemap);
            let range = caps.get(0).unwrap().range();
            result.replace_range(range.start..range.end, &subst_result);
        } else {
            break;
        }
    }
    result
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
