wasm := "target/wasm32-wasip2/release/component_filesystem.wasm"
act := env("ACT", "npx @actcore/act")
oras := env("ORAS", "oras")
registry := env("OCI_REGISTRY", "ghcr.io/actpkg")
port := `npx get-port-cli`
addr := "[::1]:" + port
baseurl := "http://" + addr

init:
    wit-deps

setup: init
    prek install

build:
    cargo build --target wasm32-wasip2 --release

test:
    #!/usr/bin/env bash
    set -euo pipefail
    TEST_DIR=$(mktemp -d)
    {{act}} run {{wasm}} --http --listen "{{addr}}" --allow-dir "/test:$TEST_DIR" &
    trap "kill $!; rm -rf $TEST_DIR" EXIT
    npx wait-on -t 180s {{baseurl}}/info
    npx @orangeopensource/hurl --test --variable "baseurl={{baseurl}}" --variable "test_dir=/test" e2e/*.hurl

publish:
    #!/usr/bin/env bash
    set -euo pipefail
    INFO=$({{act}} info {{wasm}} --format json)
    NAME=$(echo "$INFO" | jq -r .name)
    VERSION=$(echo "$INFO" | jq -r .version)
    DESC=$(echo "$INFO" | jq -r .description)
    if {{oras}} manifest fetch "{{registry}}/$NAME:$VERSION" >/dev/null 2>&1; then
      echo "$NAME:$VERSION already published, skipping"
      exit 0
    fi
    SOURCE=$(git remote get-url origin 2>/dev/null | sed 's/\.git$//' | sed 's|git@github.com:|https://github.com/|' || echo "")
    OUTPUT=$({{oras}} push "{{registry}}/$NAME:$VERSION" \
      --artifact-type application/wasm \
      --annotation "org.opencontainers.image.version=$VERSION" \
      --annotation "org.opencontainers.image.description=$DESC" \
      --annotation "org.opencontainers.image.source=$SOURCE" \
      "{{wasm}}:application/wasm" 2>&1)
    echo "$OUTPUT"
    DIGEST=$(echo "$OUTPUT" | grep "^Digest:" | awk '{print $2}')
    {{oras}} tag "{{registry}}/$NAME:$VERSION" latest
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
      echo "image={{registry}}/$NAME" >> "$GITHUB_OUTPUT"
      echo "digest=$DIGEST" >> "$GITHUB_OUTPUT"
    fi
