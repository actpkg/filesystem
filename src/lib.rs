use act_sdk::prelude::*;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Result of a write/append operation.
#[derive(Serialize, JsonSchema)]
struct WriteResult {
    path: String,
    bytes_written: u64,
}

/// A single directory entry.
#[derive(Serialize, JsonSchema)]
struct DirEntry {
    /// File or directory name (relative path when listing recursively).
    name: String,
    /// One of `file`, `directory`, or `symlink`.
    #[serde(rename = "type")]
    entry_type: String,
    /// Size in bytes (omitted when metadata is unavailable).
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

/// Result of `list_directory`.
///
/// An object rather than a bare array so the listing can carry what the
/// traversal had to leave out: entries it stopped at the cap, directories it
/// could not read. Both are facts an agent can act on, not errors — a
/// partial answer that says so beats one that pretends to be complete.
#[derive(Serialize, JsonSchema)]
struct DirectoryListing {
    entries: Vec<DirEntry>,
    /// True when the traversal stopped at `max_entries` before finishing.
    truncated: bool,
    /// Number of subdirectories that could not be read (permission refused
    /// by the host) and were skipped.
    skipped_dirs: u32,
}

/// Result of a move/rename operation.
#[derive(Serialize, JsonSchema)]
struct MoveResult {
    from: String,
    to: String,
}

/// Result of a copy operation.
#[derive(Serialize, JsonSchema)]
struct CopyResult {
    from: String,
    to: String,
    bytes_copied: u64,
}

/// Result of a delete operation.
#[derive(Serialize, JsonSchema)]
struct DeleteResult {
    deleted: String,
}

#[act_component]
mod component {
    use super::*;

    /// Read a text file and return it as a content-part with a guessed text MIME type.
    #[act_tool(description = "Read the contents of a text file", read_only)]
    fn read_file(#[doc = "Path to the file to read"] path: String) -> ActResult<Content> {
        let content = fs::read_to_string(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ActError::not_found(format!("File not found: {path}")),
            _ => io_error(e, "Read", &path),
        })?;
        // read_file always returns text; fall back to text/plain for unknown extensions.
        let mime = match guess_mime(&path) {
            "application/octet-stream" => "text/plain",
            m => m,
        };
        Ok(Content(mime, content.into_bytes()))
    }

    /// Read a binary file and return raw bytes with detected MIME type.
    #[act_tool(
        description = "Read a binary file and return its raw content with detected MIME type",
        read_only
    )]
    fn read_binary_file(#[doc = "Path to the binary file"] path: String) -> ActResult<Content> {
        let data = fs::read(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ActError::not_found(format!("File not found: {path}")),
            _ => io_error(e, "Read", &path),
        })?;
        Ok(Content(guess_mime(&path), data))
    }

    /// Write content to a file (creates or overwrites). Parent directories are created automatically.
    #[act_tool(description = "Write text content to a file (creates new or overwrites existing)")]
    fn write_file(
        #[doc = "Path to write to"] path: String,
        #[doc = "Content to write"] content: String,
    ) -> ActResult<WriteResult> {
        if let Some(parent) = Path::new(&path).parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|e| io_error(e, "Create directories", &parent.display().to_string()))?;
        }
        fs::write(&path, &content).map_err(|e| io_error(e, "Write", &path))?;
        Ok(WriteResult {
            bytes_written: content.len() as u64,
            path,
        })
    }

    /// Append content to a file (creates if missing). Avoids reading the whole file.
    #[act_tool(description = "Append text content to a file")]
    fn append_file(
        #[doc = "Path to append to"] path: String,
        #[doc = "Content to append"] content: String,
    ) -> ActResult<WriteResult> {
        use io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| io_error(e, "Open", &path))?;
        file.write_all(content.as_bytes())
            .map_err(|e| io_error(e, "Write", &path))?;
        Ok(WriteResult {
            bytes_written: content.len() as u64,
            path,
        })
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
        #[doc = "Stop after this many entries and report truncated (default 10000)"]
        max_entries: Option<u32>,
    ) -> ActResult<DirectoryListing> {
        let recursive = recursive.unwrap_or(false);
        let max_depth = max_depth.unwrap_or(10);
        let mut walk = Traversal {
            base: PathBuf::from(&path),
            glob: glob.clone(),
            max_depth,
            max_entries: max_entries.unwrap_or(10_000).max(1) as usize,
            truncated: false,
            skipped_dirs: 0,
            results: Vec::new(),
        };

        collect_entries(&mut walk, Path::new(&path), recursive, 0)?;
        walk.results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(DirectoryListing {
            entries: walk.results,
            truncated: walk.truncated,
            skipped_dirs: walk.skipped_dirs,
        })
    }

    /// Move or rename a file/directory.
    #[act_tool(description = "Move or rename a file or directory")]
    fn move_file(
        #[doc = "Source path"] source: String,
        #[doc = "Destination path"] destination: String,
    ) -> ActResult<MoveResult> {
        fs::rename(&source, &destination)
            .map_err(|e| io_error(e, "Move", &format!("{source} -> {destination}")))?;
        Ok(MoveResult {
            from: source,
            to: destination,
        })
    }

    /// Copy a file (filesystem-optimized, supports reflinks).
    #[act_tool(description = "Copy a file to a new location")]
    fn copy_file(
        #[doc = "Source file path"] source: String,
        #[doc = "Destination file path"] destination: String,
    ) -> ActResult<CopyResult> {
        let bytes = fs::copy(&source, &destination)
            .map_err(|e| io_error(e, "Copy", &format!("{source} -> {destination}")))?;
        Ok(CopyResult {
            from: source,
            to: destination,
            bytes_copied: bytes,
        })
    }

    /// Delete a file.
    #[act_tool(description = "Delete a file", destructive)]
    fn delete_file(#[doc = "Path to the file to delete"] path: String) -> ActResult<DeleteResult> {
        fs::remove_file(&path).map_err(|e| io_error(e, "Delete", &path))?;
        Ok(DeleteResult { deleted: path })
    }

    /// Delete a directory (must be empty, or use recursive).
    #[act_tool(description = "Delete a directory (optionally recursive)", destructive)]
    fn delete_directory(
        #[doc = "Path to the directory to delete"] path: String,
        #[doc = "Whether to delete recursively (including contents). Default false."]
        recursive: Option<bool>,
    ) -> ActResult<DeleteResult> {
        if recursive.unwrap_or(false) {
            fs::remove_dir_all(&path).map_err(|e| io_error(e, "Delete", &path))?;
        } else {
            fs::remove_dir(&path).map_err(|e| match e.kind() {
                io::ErrorKind::PermissionDenied => io_error(e, "Delete", &path),
                _ => ActError::internal(format!("Delete error (directory not empty?): {e}")),
            })?;
        }
        Ok(DeleteResult { deleted: path })
    }
}

/// The error an I/O failure should become, once a caller has handled the
/// cases it has something specific to say about (a missing file, usually).
///
/// A refusal is not a failure of this component. The host decides what may be
/// touched, and when it says no — `PermissionDenied`, which is where WASI's
/// `EACCES` and `EPERM` both land — the agent should hear
/// `std:capability-denied`, a kind it can act on: ask for access, or try a
/// path it was given. `std:internal` would send it looking for a bug here.
///
/// One function rather than an arm per call site, because the arm per call
/// site is how eight of the nine tools came to lack it.
fn io_error(e: io::Error, action: &str, subject: &str) -> ActError {
    match e.kind() {
        io::ErrorKind::PermissionDenied => {
            ActError::capability_denied(format!("Permission denied: {subject}"))
        }
        _ => ActError::internal(format!("{action} error: {e}")),
    }
}

/// Traversal bookkeeping shared across the recursion of `collect_entries`:
/// the entry cap that bounds a listing, and the counters that make what was
/// left out visible instead of silent.
struct Traversal {
    base: PathBuf,
    glob: Option<String>,
    max_depth: u32,
    max_entries: usize,
    truncated: bool,
    skipped_dirs: u32,
    results: Vec<DirEntry>,
}

/// Collect directory entries, optionally filtering by glob and recursing.
///
/// Recursion stops at the entry cap (`truncated`), and directories the host
/// refuses to read are counted in `skipped_dirs` rather than failing the
/// whole listing — a tree that happens to contain an unreadable subtree must
/// still be listable. The root directory itself is the exception: failing to
/// open it is what the caller asked about, so that stays an error.
fn collect_entries(walk: &mut Traversal, dir: &Path, recursive: bool, depth: u32) -> ActResult<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            // Subdirectories the host refuses to read are skipped and
            // counted; the root is the one failure that is the answer.
            if depth > 0 && e.kind() == io::ErrorKind::PermissionDenied {
                walk.skipped_dirs += 1;
                return Ok(());
            }
            if depth == 0 && e.kind() == io::ErrorKind::NotFound {
                return Err(ActError::not_found(format!(
                    "Directory not found: {}",
                    dir.display()
                )));
            }
            return Err(io_error(e, "Read dir", &dir.display().to_string()));
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            walk.skipped_dirs += 1;
            continue;
        };
        let metadata = entry.metadata().ok();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());

        if walk.results.len() >= walk.max_entries {
            walk.truncated = true;
            return Ok(());
        }

        let matches = walk.glob.as_deref().is_none_or(|g| glob_match(g, &name));

        if matches {
            let display_path = if recursive {
                entry
                    .path()
                    .strip_prefix(&walk.base)
                    .unwrap_or(&entry.path())
                    .to_string_lossy()
                    .to_string()
            } else {
                name.clone()
            };

            let entry_type = if is_dir {
                "directory"
            } else if metadata.as_ref().is_some_and(|m| m.is_symlink()) {
                "symlink"
            } else {
                "file"
            };

            walk.results.push(DirEntry {
                name: display_path,
                entry_type: entry_type.to_string(),
                size: metadata.as_ref().map(|m| m.len()),
            });
        }

        if recursive && is_dir && depth < walk.max_depth {
            collect_entries(walk, &entry.path(), recursive, depth + 1)?;
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

fn guess_mime(path: &str) -> &'static str {
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
}
