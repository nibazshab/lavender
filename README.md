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

```sh
# POST
curl -d t="text" 127.0.0.1:8080/test
curl -d "text" 127.0.0.1:8080
cat /etc/hosts | curl --data-binary @- 127.0.0.1:8080/test
cat /etc/hosts | curl -F f=@- 127.0.0.1:8080

# POST
curl -F f=@a.jpg 127.0.0.1:8080/file/
# DELETE
curl -X DELETE 127.0.0.1:8080/file/test -H 'token: 2A9B3F692B1715A6'
```

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
