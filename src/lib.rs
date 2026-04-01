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

    /// Write content to a file (creates or overwrites). Parent directories are created automatically.
    #[act_tool(description = "Write text content to a file (creates new or overwrites existing)")]
    fn write_file(
        #[doc = "Path to write to"] path: String,
        #[doc = "Content to write"] content: String,
    ) -> ActResult<String> {
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

    /// Append content to a file (creates if missing). Avoids reading the whole file.
    #[act_tool(description = "Append text content to a file")]
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

    /// List directory contents with optional glob filter and recursion.
    #[act_tool(
        description = "List files and directories. Supports glob filtering and recursive search.",
        read_only
    )]
    fn list_directory(
        #[doc = "Path to the directory to list"] path: String,
        #[doc = "Glob pattern to filter by name (e.g. '*.rs', 'test_*')"] glob: Option<String>,
        #[doc = "Recurse into subdirectories (default false)"] recursive: Option<bool>,
        #[doc = "Maximum depth when recursive (default 10)"] max_depth: Option<u32>,
    ) -> ActResult<Vec<serde_json::Value>> {
        let recursive = recursive.unwrap_or(false);
        let max_depth = max_depth.unwrap_or(10);
        let mut items = Vec::new();

        collect_entries(
            Path::new(&path),
            Path::new(&path),
            glob.as_deref(),
            recursive,
            0,
            max_depth,
            &mut items,
        )?;

        items.sort_by(|a, b| {
            a.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        });

        Ok(items)
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

    /// Copy a file (filesystem-optimized, supports reflinks).
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
}

/// Collect directory entries, optionally filtering by glob and recursing.
fn collect_entries(
    base: &Path,
    dir: &Path,
    glob: Option<&str>,
    recursive: bool,
    depth: u32,
    max_depth: u32,
    results: &mut Vec<serde_json::Value>,
) -> ActResult<()> {
    let entries = fs::read_dir(dir).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => {
            ActError::not_found(format!("Directory not found: {}", dir.display()))
        }
        _ => ActError::internal(format!("Read dir error: {e}")),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| ActError::internal(format!("Entry error: {e}")))?;
        let metadata = entry.metadata().ok();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());

        let matches = glob.is_none_or(|g| glob_match(g, &name));

        if matches {
            let display_path = if recursive {
                entry
                    .path()
                    .strip_prefix(base)
                    .unwrap_or(&entry.path())
                    .to_string_lossy()
                    .to_string()
            } else {
                name.clone()
            };

            let file_type = if is_dir {
                "directory"
            } else if metadata.as_ref().is_some_and(|m| m.is_symlink()) {
                "symlink"
            } else {
                "file"
            };

            let mut item = serde_json::json!({
                "name": display_path,
                "type": file_type,
            });
            if let Some(meta) = &metadata {
                item.as_object_mut()
                    .unwrap()
                    .insert("size".into(), serde_json::json!(meta.len()));
            }
            results.push(item);
        }

        if recursive && is_dir && depth < max_depth {
            collect_entries(
                base,
                &entry.path(),
                glob,
                recursive,
                depth + 1,
                max_depth,
                results,
            )?;
        }
    }

    Ok(())
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
