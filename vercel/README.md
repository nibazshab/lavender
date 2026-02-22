## 说明

正常版本，支持 Windows/Mac/Linux/FreeBSD

- systemd service

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

- `/xx` - get/post
- `/` - post
- `/-/xx` - get

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

## 编译

```sh
cargo check
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

```sh
cargo build --verbose --release
```
