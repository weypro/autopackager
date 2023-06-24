# 自动打包器 | Auto Packager

## 说明
能够自动解析yaml文件并执行对应的任务

支持以下命令
- copy: 复制指定文件夹中的文件到目标路径，支持解析.gitignore
- replace: 替换指定文件中正则匹配到的字符串
- run: 运行指定命令，按平台分为cmd和shell

yaml文件中的相对路径应该是相对于文件路径本身来说。
