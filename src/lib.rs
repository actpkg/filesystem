use act_sdk::prelude::*;
use std::fs;
use std::io;
use std::path::Path;

act_sdk::embed_skill!("skill/");

#[act_component]
mod component {
    use super::*;

    /// Read a text file.
    #[act_tool(description = "Read the contents of a text file", read_only)]
    fn read_file(#[doc = "Path to the file to read"] path: String) -> ActResult<String> {
        fs::read_to_string(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ActError::not_found(format!("File not found: {path}")),
            io::ErrorKind::PermissionDenied => ActError::new(
                "std:capability-denied",
                format!("Permission denied: {path}"),
            ),
            _ => ActError::internal(format!("Read error: {e}")),
        })
    }

    /// Read a binary file and return raw bytes with appropriate MIME type.
    #[act_tool(
        description = "Read a binary file and return its raw content with detected MIME type",
        read_only
    )]
    async fn read_binary_file(
        #[doc = "Path to the binary file"] path: String,
        ctx: &mut ActContext,
    ) -> ActResult<()> {
        let data = fs::read(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ActError::not_found(format!("File not found: {path}")),
            _ => ActError::internal(format!("Read error: {e}")),
        })?;
        let mime = guess_mime(&path);
        ctx.send_content(data, Some(mime), vec![]);
        Ok(())
    }

    /// Read multiple files at once.
    #[act_tool(
        description = "Read multiple text files at once, returning a map of path to content",
        read_only
    )]
    fn read_multiple_files(
        #[doc = "List of file paths to read"] paths: Vec<String>,
    ) -> ActResult<String> {
        let mut results = serde_json::Map::new();
        for path in &paths {
            match fs::read_to_string(path) {
                Ok(content) => {
                    results.insert(path.clone(), serde_json::json!(content));
                }
                Err(e) => {
                    results.insert(path.clone(), serde_json::json!({"error": e.to_string()}));
                }
            }
        }
        serde_json::to_string_pretty(&results)
            .map_err(|e| ActError::internal(format!("JSON error: {e}")))
    }

    /// Write content to a file (creates or overwrites).
    #[act_tool(description = "Write text content to a file (creates new or overwrites existing)")]
    fn write_file(
        #[doc = "Path to write to"] path: String,
        #[doc = "Content to write"] content: String,
    ) -> ActResult<String> {
        // Create parent directories if needed
        if let Some(parent) = Path::new(&path).parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|e| ActError::internal(format!("Cannot create directories: {e}")))?;
        }
        fs::write(&path, &content).map_err(|e| ActError::internal(format!("Write error: {e}")))?;
        Ok(serde_json::json!({
            "path": path,
            "bytes_written": content.len(),
        })
        .to_string())
    }

    /// Append content to a file.
    #[act_tool(description = "Append text content to an existing file")]
    fn append_file(
        #[doc = "Path to append to"] path: String,
        #[doc = "Content to append"] content: String,
    ) -> ActResult<String> {
        use io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| ActError::internal(format!("Open error: {e}")))?;
        file.write_all(content.as_bytes())
            .map_err(|e| ActError::internal(format!("Write error: {e}")))?;
        Ok(serde_json::json!({
            "path": path,
            "bytes_appended": content.len(),
        })
        .to_string())
    }

    /// List directory contents.
    #[act_tool(
        description = "List files and directories in a path with metadata",
        read_only
    )]
    fn list_directory(#[doc = "Path to the directory to list"] path: String) -> ActResult<String> {
        let entries = fs::read_dir(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ActError::not_found(format!("Directory not found: {path}")),
            _ => ActError::internal(format!("Read dir error: {e}")),
        })?;

        let mut items: Vec<serde_json::Value> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ActError::internal(format!("Entry error: {e}")))?;
            let metadata = entry.metadata().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = if metadata.as_ref().is_some_and(|m| m.is_dir()) {
                "directory"
            } else if metadata.as_ref().is_some_and(|m| m.is_symlink()) {
                "symlink"
            } else {
                "file"
            };

            let mut item = serde_json::json!({
                "name": name,
                "type": file_type,
            });

            if let Some(meta) = &metadata {
                item.as_object_mut()
                    .unwrap()
                    .insert("size".into(), serde_json::json!(meta.len()));
            }
            items.push(item);
        }

        // Sort by name
        items.sort_by(|a, b| {
            a.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        });

        serde_json::to_string_pretty(&items)
            .map_err(|e| ActError::internal(format!("JSON error: {e}")))
    }

    /// Get recursive directory tree as JSON.
    #[act_tool(
        description = "Get a recursive directory tree as JSON structure",
        read_only
    )]
    fn directory_tree(
        #[doc = "Root path for the tree"] path: String,
        #[doc = "Maximum depth to recurse (default 3)"] max_depth: Option<u32>,
    ) -> ActResult<String> {
        let max = max_depth.unwrap_or(3);
        let tree = build_tree(Path::new(&path), 0, max)?;
        serde_json::to_string_pretty(&tree)
            .map_err(|e| ActError::internal(format!("JSON error: {e}")))
    }

    /// Get detailed file/directory info.
    #[act_tool(
        description = "Get detailed metadata for a file or directory (size, type, permissions)",
        read_only
    )]
    fn get_file_info(#[doc = "Path to inspect"] path: String) -> ActResult<String> {
        let metadata = fs::metadata(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ActError::not_found(format!("Not found: {path}")),
            _ => ActError::internal(format!("Metadata error: {e}")),
        })?;

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_symlink() {
            "symlink"
        } else {
            "file"
        };

        let mut info = serde_json::json!({
            "path": path,
            "type": file_type,
            "size": metadata.len(),
            "readonly": metadata.permissions().readonly(),
        });

        // Try to read symlink target
        if metadata.is_symlink()
            && let Ok(target) = fs::read_link(&path)
        {
            info.as_object_mut().unwrap().insert(
                "symlink_target".into(),
                serde_json::json!(target.to_string_lossy()),
            );
        }

        serde_json::to_string_pretty(&info)
            .map_err(|e| ActError::internal(format!("JSON error: {e}")))
    }

    /// Create a directory (and parent directories).
    #[act_tool(description = "Create a directory (and any missing parent directories)")]
    fn create_directory(
        #[doc = "Path of the directory to create"] path: String,
    ) -> ActResult<String> {
        fs::create_dir_all(&path)
            .map_err(|e| ActError::internal(format!("Create dir error: {e}")))?;
        Ok(serde_json::json!({ "created": path }).to_string())
    }

    /// Move or rename a file/directory.
    #[act_tool(description = "Move or rename a file or directory")]
    fn move_file(
        #[doc = "Source path"] source: String,
        #[doc = "Destination path"] destination: String,
    ) -> ActResult<String> {
        fs::rename(&source, &destination)
            .map_err(|e| ActError::internal(format!("Move error: {e}")))?;
        Ok(serde_json::json!({
            "from": source,
            "to": destination,
        })
        .to_string())
    }

    /// Copy a file.
    #[act_tool(description = "Copy a file to a new location")]
    fn copy_file(
        #[doc = "Source file path"] source: String,
        #[doc = "Destination file path"] destination: String,
    ) -> ActResult<String> {
        let bytes = fs::copy(&source, &destination)
            .map_err(|e| ActError::internal(format!("Copy error: {e}")))?;
        Ok(serde_json::json!({
            "from": source,
            "to": destination,
            "bytes_copied": bytes,
        })
        .to_string())
    }

    /// Delete a file.
    #[act_tool(description = "Delete a file", destructive)]
    fn delete_file(#[doc = "Path to the file to delete"] path: String) -> ActResult<String> {
        fs::remove_file(&path).map_err(|e| ActError::internal(format!("Delete error: {e}")))?;
        Ok(serde_json::json!({ "deleted": path }).to_string())
    }

    /// Delete a directory (must be empty, or use recursive).
    #[act_tool(description = "Delete a directory (optionally recursive)", destructive)]
    fn delete_directory(
        #[doc = "Path to the directory to delete"] path: String,
        #[doc = "Whether to delete recursively (including contents). Default false."]
        recursive: Option<bool>,
    ) -> ActResult<String> {
        if recursive.unwrap_or(false) {
            fs::remove_dir_all(&path)
                .map_err(|e| ActError::internal(format!("Delete error: {e}")))?;
        } else {
            fs::remove_dir(&path).map_err(|e| {
                ActError::internal(format!("Delete error (directory not empty?): {e}"))
            })?;
        }
        Ok(serde_json::json!({ "deleted": path }).to_string())
    }

    /// Search for files by name pattern (simple glob: * matches any chars).
    #[act_tool(
        description = "Search for files by name pattern (glob-like: * matches any sequence of characters)",
        read_only
    )]
    fn search_files(
        #[doc = "Directory to search in"] path: String,
        #[doc = "File name pattern (* as wildcard, e.g. '*.rs', 'test_*')"] pattern: String,
        #[doc = "Maximum depth to search (default 10)"] max_depth: Option<u32>,
    ) -> ActResult<String> {
        let max = max_depth.unwrap_or(10);
        let mut results = Vec::new();
        search_recursive(Path::new(&path), &pattern, 0, max, &mut results)?;

        serde_json::to_string_pretty(&results)
            .map_err(|e| ActError::internal(format!("JSON error: {e}")))
    }

    /// Check if a file or directory exists.
    #[act_tool(
        description = "Check if a file or directory exists at the given path",
        read_only
    )]
    fn exists(#[doc = "Path to check"] path: String) -> ActResult<String> {
        let exists = Path::new(&path).exists();
        let is_file = Path::new(&path).is_file();
        let is_dir = Path::new(&path).is_dir();
        Ok(serde_json::json!({
            "path": path,
            "exists": exists,
            "is_file": is_file,
            "is_directory": is_dir,
        })
        .to_string())
    }
}

fn build_tree(path: &Path, depth: u32, max_depth: u32) -> ActResult<serde_json::Value> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    if path.is_file() {
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        return Ok(serde_json::json!({
            "name": name,
            "type": "file",
            "size": size,
        }));
    }

    if depth >= max_depth {
        return Ok(serde_json::json!({
            "name": name,
            "type": "directory",
            "truncated": true,
        }));
    }

    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            children.push(build_tree(&entry.path(), depth + 1, max_depth)?);
        }
    }

    Ok(serde_json::json!({
        "name": name,
        "type": "directory",
        "children": children,
    }))
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let name = name.to_lowercase();
    let mut p = pattern.as_str();
    let mut n = name.as_str();

    loop {
        if p.is_empty() {
            return n.is_empty();
        }
        if p.starts_with('*') {
            p = &p[1..];
            if p.is_empty() {
                return true;
            }
            // Try matching * against progressively more of n
            for i in 0..=n.len() {
                if glob_match(p, &n[i..]) {
                    return true;
                }
            }
            return false;
        }
        if n.is_empty() {
            return false;
        }
        if p.starts_with('?') || p.as_bytes()[0] == n.as_bytes()[0] {
            p = &p[1..];
            n = &n[1..];
        } else {
            return false;
        }
    }
}

fn guess_mime(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "md" => "text/markdown",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    depth: u32,
    max_depth: u32,
    results: &mut Vec<String>,
) -> ActResult<()> {
    if depth > max_depth {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| ActError::internal(format!("Search error at {}: {e}", dir.display())))?;

    for entry in entries {
        let entry = entry.map_err(|e| ActError::internal(format!("Entry error: {e}")))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        if glob_match(pattern, &name) {
            results.push(path.to_string_lossy().to_string());
        }

        if path.is_dir() {
            search_recursive(&path, pattern, depth + 1, max_depth, results)?;
        }
    }

    Ok(())
}
