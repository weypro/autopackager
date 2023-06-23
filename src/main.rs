use anyhow::{anyhow, Result};
mod packager_command;

// 测试代码
fn main() {
    let config = packager_command::parse_commands_from_yaml(&String::from("config.yaml")).unwrap();

    // 打印Config对象的内容，验证反序列化是否正确
    println!("{:#?}", config);

    packager_command::execute_commands(&config.command).unwrap();
}

