## 待办事项

没有钱续费服务器，开始研究一些邪门歪道的白嫖方案，可惜 LeanCloud 停止服务了

- [ ] 远程数据库
- [ ] Vercel 部署

## 说明

理论上应该支持全平台，测试过 Windows/Mac/Linux/FreeBSD 都能正常编译运行

### 自启动

```ini
[Unit]
Description=webnote service
[Service]
Environment=PORT=10003 DATA_DIR=/usr/local/webnote
ExecStart=/usr/local/webnote/webnote
Restart=on-failure
[Install]
WantedBy=multi-user.target
```

### 编译流程

1. 检查代码

```sh
cargo check
cargo test
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

2. 编译文件

```sh
cargo build --verbose --release
```

## API

- `/{id}` GET/POST
- `/` GET/POST
- `/-/{id}` GET

```sh
# POST
curl -d t="text" 127.0.0.1:8080/test
curl -d "text" 127.0.0.1:8080
cat /etc/hosts | curl --data-binary @- 127.0.0.1:8080/test
cat /etc/hosts | curl -F f=@- 127.0.0.1:8080
```

- `/b/` GET/POST
- `/b/{id}` GET/DELETE

```sh
# POST
curl -F f=@a.jpg 127.0.0.1:8080/b/
# DELETE
curl -X DELETE 127.0.0.1:8080/b/test -H 'token: 2A9B3F692B1715A6'
```

## 参考

- pereorga/minimalist-web-notepad

## 题外话

本质上这只是一个练手项目，用来学习用的

发展经过 php/mysql -> go/file -> go/sqlite -> rust/sqlite
