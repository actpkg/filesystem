wasm := "target/wasm32-wasip2/release/component_filesystem.wasm"
act := env("ACT", "act")
port := `python3 -c 'import socket; s=socket.socket(socket.AF_INET, socket.SOCK_STREAM); s.bind(("", 0)); print(s.getsockname()[1]); s.close()'`
addr := "[::1]:" + port
baseurl := "http://" + addr

build:
    cargo build --target wasm32-wasip2 --release

test:
    #!/usr/bin/env bash
    TEST_DIR=$(mktemp -d)
    {{act}} serve {{wasm}} --listen "{{addr}}" &
    trap "kill $!; rm -rf $TEST_DIR" EXIT
    npx wait-on {{baseurl}}/info
    hurl --test --variable "baseurl={{baseurl}}" --variable "test_dir=$TEST_DIR" e2e/*.hurl
