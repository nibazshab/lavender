## 说明

本质上这只是一个练手项目，用来学习用的

### 1. vercel serverless 版本

api

- `/xx` - get/post
- `/` - post
- `/d/xx` - get

env

- `DATABASE_URL` - 数据库连接字符串
- `BASE_URL` - 网站 URL

### 2. server 正常版本

在 serverless 的基础记事本功能外，额外包含了文件柜功能

api

- `/xx` - get/post
- `/` - post
- `/d/xx` - get
- `/file/` - get/post
- `/file/xx` - get/delete

env

- `PORT` - 监听端口号
- `BASE_URL` - 网站 URL

systemd

```ini
[Unit]
Description=lavender service
[Service]
Environment=PORT=10003 BASE_URL=https://www.example.com
ExecStart=/usr/local/lavender/lavender
Restart=on-failure
[Install]
WantedBy=multi-user.target
```

## 编译

```sh
cargo check
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

```sh
cargo build --verbose --release --no-default-features --features server
```

## 其他

### 1. 大饼

- [x] 远程数据库 PostgreSQL
- [x] Vercel 部署

### 2. 参考

- pereorga/minimalist-web-notepad
