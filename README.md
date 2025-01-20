# Web Notepad

这是一个简单的网页记事本，用于临时记录一些内容

### 快速上手

独立的二进制文件，直接运行即可，默认监听 10003 端口，使用 SQLite 存储内容，数据和日志默认位于 webnote_data 目录中

```sh
./webnote
```

### 构建说明

所需软件包：go, musl

运行 Makefile 文件即可，go 使用包管理器或任意方式安装，musl 将在构建过程中自动安装

```sh
make
```

### 使用说明

命令行可以接收的参数

| 参数    | 默认值          | 描述              |
|-------|--------------|-----------------|
| -port | 10003        | 程序监听的端口号        |
| -path | webnote_data | 数据目录（相对程序文件的路径） |

### API

- ___POST /{uid}___

请求：application/x-www-form-urlencoded，无返回

body：`t` 文本内容

- ___GET /{sid}___

返回该链接所对应的文本内容

## PLAN-B

- [x] 解决 favicon.ico 被重定向的问题
- [x] 变更相对路径为绝对路径
- [x] 自定义端口号
- [x] Ctrl R 全局替换文字（不可撤销）
- [x] 指定数据存储位置
- [x] 支持绝对路径的数据目录

## 许可证

Powered by https://github.com/pereorga/minimalist-web-notepad

MIT © ZShab Niba
