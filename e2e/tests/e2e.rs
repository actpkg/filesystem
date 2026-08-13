//! Drive the packed component through `act run --mcp` with a real MCP client.
//!
//! This replaces the python fastmcp/pytest suite that used to live beside it:
//! the tests observe exactly what an agent observes, over the same client
//! stack (`rmcp`) the host bridge itself is built on.
//!
//! `filesystem` is stateful across *files*, not just across calls within a
//! process, so every test gets its own `act` process AND its own private
//! directory (the python `client` + `tmp_path` fixture pair's guarantee,
//! kept): the grant is scoped to that directory and nothing else.
//!
//! Env: WASM — path to the packed component (default: the component's
//!      release build output);
//!      ACT  — the act invocation (default `act`; `npx @actcore/act`, the
//!             component justfile's default, also works — whitespace-split,
//!             like the shlex.split the python conftest did).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex as AsyncMutex;

/// `().serve(transport)` hands back the client-role service running over the
/// child process: role first, the unit client handler second.
type Client = rmcp::service::RunningService<rmcp::service::RoleClient, ()>;

fn wasm_path() -> PathBuf {
    PathBuf::from(std::env::var("WASM").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/wasm32-wasip2/release/component_filesystem.wasm"
        )
        .into()
    }))
}

/// The ACT invocation, honouring the same override the component justfile
/// uses. Its default there is `npx @actcore/act` — two words — which cannot
/// be `argv[0]` for a non-shell spawn, so the value is whitespace-split into
/// program + leading args. Quoted paths with spaces are not a form this
/// fleet passes through `ACT`; a full shlex is deliberately not pulled in.
fn act_argv() -> Vec<String> {
    std::env::var("ACT")
        .unwrap_or_else(|_| "act".into())
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// A private directory per test. The python suite leaned on pytest's
/// `tmp_path` for this; the stdlib stand-in is a unique name under the same
/// temp root. `filesystem` state lives in files across calls, so sharing a
/// tree between tests would be real cross-test coupling, not a cosmetic one.
fn private_dir(test: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock is set")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "act-filesystem-e2e-{}-{}-{nanos}",
        test,
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create the test's private directory");
    dir
}

/// A sibling of the private directory — outside the grant, but still under
/// the temp root the tests are allowed to litter. The python suite built the
/// same shape (`tmp_path.parent / f"{tmp_path.name}-outside.*"`); the files
/// are never created, the refusal must happen before existence matters.
fn outside_path(dir: &Path, suffix: &str) -> PathBuf {
    dir.parent()
        .expect("private dir has a parent")
        .join(format!("{}-outside{suffix}", dir.file_name().unwrap().to_string_lossy()))
}

/// Spawn `act run <wasm> --mcp` with the grant this test's directory.
///
/// Grants are NOT optional: the default policy mode is `ask` and a headless
/// run degrades it to deny. Grant shape carried verbatim from the python
/// conftest (mode `allowlist`, `rw`, path = the private directory, no `/**`
/// suffix needed — `wasi:filesystem` treats it as a subtree root), and never
/// above the declared ceiling (`path = "**"`, act.toml: "Path scope is
/// delegated to the host policy").
fn act_command(dir: &Path) -> tokio::process::Command {
    let argv = act_argv();
    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.arg("run").arg(wasm_path()).arg("--mcp");
    let grant = json!({
        "wasi:filesystem": {
            "mode": "allowlist",
            "allow": [{ "path": dir.to_string_lossy(), "mode": "rw" }],
        }
    });
    cmd.args(["--grant", &grant.to_string()]);
    cmd
}

fn spawn_transport(dir: &Path) -> TokioChildProcess {
    TokioChildProcess::new(act_command(dir)).expect("spawn act run --mcp")
}

/// Spawn with stderr captured: the audit trail (refusals, per-call rollup)
/// writes there unconditionally — RUST_LOG never silences it.
fn spawn_with_captured_stderr(dir: &Path) -> (TokioChildProcess, Arc<AsyncMutex<String>>) {
    let (transport, stderr) = TokioChildProcess::builder(act_command(dir))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn act run --mcp with piped stderr");

    let captured = Arc::new(AsyncMutex::new(String::new()));
    let sink = captured.clone();
    let mut lines = BufReader::new(stderr.expect("stderr was piped")).lines();
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            sink.lock().await.push_str(&line);
            sink.lock().await.push('\n');
        }
    });

    (transport, captured)
}

/// Poll the captured stderr until `needle` appears — the audit line is
/// flushed before the JSON-RPC reply, but reaching this buffer still crosses
/// a pipe and an async read.
async fn wait_for_stderr(
    captured: &Arc<AsyncMutex<String>>,
    needle: &str,
    timeout: Duration,
) -> bool {
    let start = std::time::Instant::now();
    loop {
        if captured.lock().await.contains(needle) {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn connect(dir: &Path) -> Client {
    ().serve(spawn_transport(dir))
        .await
        .expect("rmcp handshake with act run --mcp")
}

fn text_blocks(result: &rmcp::model::CallToolResult) -> Vec<String> {
    result
        .content
        .iter()
        .filter_map(|b| match b {
            rmcp::model::ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

fn first_text(result: &rmcp::model::CallToolResult) -> String {
    match result.content.first() {
        Some(rmcp::model::ContentBlock::Text(t)) => t.text.clone(),
        other => panic!("expected the first content block to be Text, got: {other:?}"),
    }
}

fn structured<'a>(result: &'a rmcp::model::CallToolResult, tool: &str) -> &'a Value {
    result
        .structured_content
        .as_ref()
        .unwrap_or_else(|| panic!("{tool} must return structured content, got: {result:?}"))
}

async fn call(client: &Client, tool: &str, args: Value) -> rmcp::model::CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(tool.to_string())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .expect("call_tool")
}

/// The kind and message of a failed call may arrive on either path: as a
/// JSON-RPC error response (`ErrorData.data`) or as an isError result
/// (`_meta`). The python conftest's `expect_error` fixture handled both; so
/// does this. `call-tool` has no `result<>` wrapper, so a guest reporting a
/// failed call can only do it through `tool-event::error` — the isError path
/// here; the JSON-RPC path stays handled for the non-guest failure modes.
async fn error_kind_of(client: &Client, params: CallToolRequestParams) -> (String, String) {
    match client.call_tool(params).await {
        Err(rmcp::ServiceError::McpError(e)) => {
            let kind = e
                .data
                .as_ref()
                .and_then(|d| d.get("dev.actcore/error-kind"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_default();
            (kind, e.message.to_string())
        }
        Ok(result) => {
            assert_eq!(result.is_error, Some(true), "call must fail: {result:?}");
            let kind = result
                .meta
                .as_ref()
                .and_then(|m| m.0.get("dev.actcore/error-kind"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_default();
            (kind, first_text(&result))
        }
        Err(other) => panic!("unexpected transport failure: {other:?}"),
    }
}

async fn expect_error(client: &Client, tool: &str, args: Value, kind: &str) {
    let (got, message) = error_kind_of(
        client,
        CallToolRequestParams::new(tool.to_string())
            .with_arguments(args.as_object().unwrap().clone()),
    )
    .await;
    assert_eq!(
        got, kind,
        "expected {kind} from {tool}, got {got:?}: {message}"
    );
}

/// Best-effort cleanup; a panicking test leaks its directory, which the
/// pytest suite tolerated too (pytest itself keeps the last few runs).
fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// The manifest probe from python test_info.py: the packed artifact must
/// declare its name. Also the fast-fail the python `wasm_path` fixture
/// provided — an unpacked wasm (raw `cargo build` output, no `act:component`
/// section) declares no ceiling, every grant is refused as "outside
/// ceiling", and the failures point anywhere but at the missing metadata.
/// The justfile's `test: build` ordering exists so this test finds a packed
/// artifact.
#[test]
fn manifest_reports_name() {
    let output = {
        let argv = act_argv();
        let mut cmd = std::process::Command::new(&argv[0]);
        cmd.args(&argv[1..]);
        cmd.args(["inspect", "component-manifest"])
            .arg(wasm_path())
            .output()
            .expect("run act inspect component-manifest")
    };
    assert!(
        output.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: Value = serde_json::from_slice(&output.stdout).expect("manifest is JSON");
    assert_eq!(
        manifest["std"]["name"], "filesystem",
        "packed manifest must carry the component name"
    );
}

// ---------------------------------------------------------------------------
// Tool inventory
// ---------------------------------------------------------------------------

#[tokio::test]
async fn component_exposes_its_tools() {
    let client = connect(&private_dir("exposes")).await;
    let tools = client.list_all_tools().await.expect("list_all_tools");
    assert!(
        !tools.is_empty(),
        "component must expose at least one tool"
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn lists_all_nine_tools() {
    let client = connect(&private_dir("nine")).await;
    let tools = client.list_all_tools().await.expect("list_all_tools");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert_eq!(names.len(), 9, "expected nine tools, got: {names:?}");
    for expected in ["read_file", "write_file", "list_directory", "copy_file", "delete_file"] {
        assert!(
            names.contains(&expected),
            "{expected} must be among the tools, got: {names:?}"
        );
    }
    client.cancel().await.ok();
}

// ---------------------------------------------------------------------------
// Read / write
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_then_read() {
    let dir = private_dir("rw");
    let client = connect(&dir).await;
    let path = dir.join("hello.txt");

    let result = call(
        &client,
        "write_file",
        json!({"path": path.to_string_lossy(), "content": "Hello, ACT!"}),
    )
    .await;
    assert_ne!(result.is_error, Some(true), "write_file failed: {result:?}");
    assert_eq!(
        structured(&result, "write_file")["bytes_written"], 11,
        "\"Hello, ACT!\" is 11 bytes"
    );

    let result = call(&client, "read_file", json!({"path": path.to_string_lossy()})).await;
    assert_ne!(result.is_error, Some(true), "read_file failed: {result:?}");
    assert_eq!(first_text(&result), "Hello, ACT!");

    client.cancel().await.ok();
    cleanup(&dir);
}

/// A missing file is a named guest error, not a JSON-RPC failure: read_file
/// reports `std:not-found` through `tool-event::error`.
#[tokio::test]
async fn read_nonexistent_file() {
    let dir = private_dir("nonexistent");
    let client = connect(&dir).await;
    let path = dir.join("nonexistent.txt");

    expect_error(&client, "read_file", json!({"path": path.to_string_lossy()}), "std:not-found")
        .await;

    client.cancel().await.ok();
    cleanup(&dir);
}

// ---------------------------------------------------------------------------
// Append / copy / delete / move
// ---------------------------------------------------------------------------

#[tokio::test]
async fn append_to_existing_file() {
    let dir = private_dir("append");
    let client = connect(&dir).await;
    let path = dir.join("append.txt");

    call(
        &client,
        "write_file",
        json!({"path": path.to_string_lossy(), "content": "hello"}),
    )
    .await;

    let result = call(
        &client,
        "append_file",
        json!({"path": path.to_string_lossy(), "content": " world"}),
    )
    .await;
    assert_ne!(result.is_error, Some(true), "append_file failed: {result:?}");
    assert_eq!(
        structured(&result, "append_file")["bytes_written"], 6,
        "\" world\" is 6 bytes"
    );

    let result = call(&client, "read_file", json!({"path": path.to_string_lossy()})).await;
    assert_eq!(first_text(&result), "hello world");

    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn copy_file() {
    let dir = private_dir("copy");
    let client = connect(&dir).await;
    let src = dir.join("copy_src.txt");
    let dst = dir.join("copy_dst.txt");

    call(
        &client,
        "write_file",
        json!({"path": src.to_string_lossy(), "content": "copy me"}),
    )
    .await;

    let result = call(
        &client,
        "copy_file",
        json!({
            "source": src.to_string_lossy(),
            "destination": dst.to_string_lossy(),
        }),
    )
    .await;
    assert_ne!(result.is_error, Some(true), "copy_file failed: {result:?}");
    let st = structured(&result, "copy_file");
    assert!(
        st["from"].as_str().unwrap().contains("copy_src.txt"),
        "from must name the source, got: {st}"
    );
    assert!(
        st["to"].as_str().unwrap().contains("copy_dst.txt"),
        "to must name the destination, got: {st}"
    );
    assert_eq!(st["bytes_copied"], 7, "\"copy me\" is 7 bytes");

    for path in [&src, &dst] {
        let result = call(&client, "read_file", json!({"path": path.to_string_lossy()})).await;
        assert_eq!(first_text(&result), "copy me", "both copies must exist");
    }

    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn delete_file() {
    let dir = private_dir("delete");
    let client = connect(&dir).await;
    let path = dir.join("to_delete.txt");

    call(
        &client,
        "write_file",
        json!({"path": path.to_string_lossy(), "content": "delete me"}),
    )
    .await;

    let result = call(&client, "delete_file", json!({"path": path.to_string_lossy()})).await;
    assert_ne!(result.is_error, Some(true), "delete_file failed: {result:?}");
    assert!(
        structured(&result, "delete_file")["deleted"]
            .as_str()
            .unwrap()
            .contains("to_delete.txt"),
        "deleted must name the file, got: {result:?}"
    );

    expect_error(&client, "read_file", json!({"path": path.to_string_lossy()}), "std:not-found")
        .await;

    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn move_file() {
    let dir = private_dir("move");
    let client = connect(&dir).await;
    let src = dir.join("src.txt");
    let dst = dir.join("dst.txt");

    call(
        &client,
        "write_file",
        json!({"path": src.to_string_lossy(), "content": "moveme"}),
    )
    .await;

    let result = call(
        &client,
        "move_file",
        json!({
            "source": src.to_string_lossy(),
            "destination": dst.to_string_lossy(),
        }),
    )
    .await;
    assert_ne!(result.is_error, Some(true), "move_file failed: {result:?}");

    let result = call(&client, "read_file", json!({"path": dst.to_string_lossy()})).await;
    assert_eq!(first_text(&result), "moveme");

    expect_error(&client, "read_file", json!({"path": src.to_string_lossy()}), "std:not-found")
        .await;

    client.cancel().await.ok();
    cleanup(&dir);
}

// ---------------------------------------------------------------------------
// list_directory
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_directory() {
    let dir = private_dir("listdir");
    let client = connect(&dir).await;
    let listdir = dir.join("listdir");
    std::fs::create_dir_all(&listdir).expect("create listdir");

    for (name, content) in [("a.txt", "a"), ("b.txt", "b")] {
        call(
            &client,
            "write_file",
            json!({"path": listdir.join(name).to_string_lossy(), "content": content}),
        )
        .await;
    }

    // list_directory returns an array, so the host does not populate
    // structured_content for it (measured in the python suite — only
    // object-shaped tool results do); the array still arrives as a
    // JSON-encoded text block, same as any other content part.
    let result = call(
        &client,
        "list_directory",
        json!({"path": listdir.to_string_lossy()}),
    )
    .await;
    assert_ne!(result.is_error, Some(true), "list_directory failed: {result:?}");
    assert!(
        result.structured_content.is_none(),
        "array-shaped results carry no structured_content, got: {result:?}"
    );
    let entries: Value =
        serde_json::from_str(&first_text(&result)).expect("entries are a JSON array");
    let names: Vec<&str> = entries
        .as_array()
        .expect("entries are an array")
        .iter()
        .map(|e| e["name"].as_str().expect("entry has a name"))
        .collect();
    assert!(names.contains(&"a.txt"), "a.txt must be listed, got: {names:?}");
    assert!(names.contains(&"b.txt"), "b.txt must be listed, got: {names:?}");

    client.cancel().await.ok();
    cleanup(&dir);
}

// ---------------------------------------------------------------------------
// Denials
// ---------------------------------------------------------------------------

/// A refusal from the host is reported as a refusal, not as a crash: the
/// kind an agent can act on (`std:capability-denied` — ask for access, try
/// another path), never `std:internal` ("this component broke", which sends
/// an agent looking for a bug that is not there). Every tool in the set is
/// exercised because the mapping had holes before — only `read_file` used
/// to map a refusal, and a directory listing the host refuses answered
/// `std:internal` too.
const DENIED: &str = "std:capability-denied";

#[tokio::test]
async fn reading_outside_the_grant_is_denied() {
    let dir = private_dir("denied-read");
    let client = connect(&dir).await;
    let outside = outside_path(&dir, ".bin");
    expect_error(
        &client,
        "read_binary_file",
        json!({"path": outside.to_string_lossy()}),
        DENIED,
    )
    .await;
    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn writing_outside_the_grant_is_denied() {
    let dir = private_dir("denied-write");
    let client = connect(&dir).await;
    let outside = outside_path(&dir, ".txt");
    expect_error(
        &client,
        "write_file",
        json!({"path": outside.to_string_lossy(), "content": "x"}),
        DENIED,
    )
    .await;
    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn appending_outside_the_grant_is_denied() {
    let dir = private_dir("denied-append");
    let client = connect(&dir).await;
    let outside = outside_path(&dir, ".log");
    expect_error(
        &client,
        "append_file",
        json!({"path": outside.to_string_lossy(), "content": "x"}),
        DENIED,
    )
    .await;
    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn listing_outside_the_grant_is_denied() {
    let dir = private_dir("denied-list");
    let client = connect(&dir).await;
    let parent = dir.parent().unwrap().to_string_lossy().into_owned();
    expect_error(&client, "list_directory", json!({"path": parent}), DENIED).await;
    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn copying_out_of_the_grant_is_denied() {
    let dir = private_dir("denied-copy");
    let client = connect(&dir).await;
    let inside = dir.join("a.txt");
    call(
        &client,
        "write_file",
        json!({"path": inside.to_string_lossy(), "content": "a"}),
    )
    .await;
    let outside = outside_path(&dir, "-copy.txt");
    expect_error(
        &client,
        "copy_file",
        json!({
            "source": inside.to_string_lossy(),
            "destination": outside.to_string_lossy(),
        }),
        DENIED,
    )
    .await;
    client.cancel().await.ok();
    cleanup(&dir);
}

#[tokio::test]
async fn deleting_outside_the_grant_is_denied() {
    let dir = private_dir("denied-delete");
    let client = connect(&dir).await;
    let outside = outside_path(&dir, "-gone.txt");
    expect_error(&client, "delete_file", json!({"path": outside.to_string_lossy()}), DENIED).await;
    client.cancel().await.ok();
    cleanup(&dir);
}

/// Beyond python parity: a refusal must also be named in the audit trail on
/// stderr — "what was refused and why" is the trail's contract, and this
/// exercises the captured-stderr plumbing the harness keeps for it. The
/// needle is the refusal reason the trail prints (`✗ deny … outside
/// ceiling`), probed with `act call`; `deny` alone would also match the
/// startup summary line, which names every class's mode.
#[tokio::test]
async fn denied_write_is_audited() {
    let dir = private_dir("denied-audit");
    let (transport, captured) = spawn_with_captured_stderr(&dir);
    let client = ().serve(transport).await.expect("rmcp handshake");
    let outside = outside_path(&dir, "-audit.txt");

    expect_error(
        &client,
        "write_file",
        json!({"path": outside.to_string_lossy(), "content": "x"}),
        DENIED,
    )
    .await;

    assert!(
        wait_for_stderr(&captured, "outside ceiling", Duration::from_secs(5)).await,
        "expected the refusal to be named in the audit trail:\n{}",
        captured.lock().await
    );

    client.cancel().await.ok();
    cleanup(&dir);
}
