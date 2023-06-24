# 自动打包器 | Auto Packager

## 说明
能够自动解析yaml文件并执行对应的任务

支持以下命令：
- copy: 复制指定文件夹中的文件到目标路径，支持解析.gitignore
- replace: 替换指定文件中正则匹配到的字符串
- run: 运行指定命令，按平台分为cmd和shell

传参说明：
```
Usage: autopackager.exe [OPTIONS] --config <CONFIG>

Options:
  -c, --config <CONFIG>
  -w, --workdir <WORKDIR>
  -h, --help               Print help
  -V, --version            Print version
```

yaml文件中的相对路径，如果是传入了workdir参数，则是相对于该路径。否则是相对于yaml文件本身路径。
