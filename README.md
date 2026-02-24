## 说明

### 1. vercel.rs

vercel serverless 版本

api

- `/xx` - get/post
- `/` - post
- `/d/xx` - get

env

- `DATABASE_URL` - 数据库连接字符串

### 2. server.rs

正常版本，支持 Windows/Mac/Linux/FreeBSD

api

- `/xx` - get/post
- `/` - post
- `/d/xx` - get
- `/file/` - get/post
- `/file/x` - get/delete

```sh
# POST
curl -d t="text" 127.0.0.1:8080/test
curl -d "text" 127.0.0.1:8080
cat /etc/hosts | curl --data-binary @- 127.0.0.1:8080/test
cat /etc/hosts | curl -F f=@- 127.0.0.1:8080

# POST
curl -F f=@a.jpg 127.0.0.1:8080/b/
# DELETE
curl -X DELETE 127.0.0.1:8080/b/test -H 'token: 2A9B3F692B1715A6'
```

env

- `PORT` - 监听端口号

Linux systemd 自启动配置文件示例

```ini
[Unit]
Description=webnote service
[Service]
Environment=PORT=10003
ExecStart=/usr/local/webnote/webnote
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

没有钱续费服务器，开始研究一些邪门歪道的白嫖方案，可惜 LeanCloud 停止服务了

- [x] 远程数据库 PostgreSQL
- [x] Vercel 部署

### 2. 参考

- pereorga/minimalist-web-notepad

### 3. 题外话

本质上这只是一个练手项目，用来学习用的

发展经过 php/mysql -> go/file -> go/sqlite -> rust/sqlite -> serverless/rust/pgsql
