---
name: filesystem
description: File system operations — read, write, list, search, copy, move, delete
metadata:
  act: {}
---

# Filesystem Component

Sandboxed file system access. All paths are relative to the component's mounted directories.

## Tools

### read_file
Read a text file. Returns content as string.

```
read_file(path: "/data/config.json")
```

### read_binary_file
Read a binary file. Returns raw bytes with detected MIME type (streaming).

### read_multiple_files
Read several files at once. Returns a map of path to content (or error).

```
read_multiple_files(paths: ["/data/a.txt", "/data/b.txt"])
→ {"/data/a.txt": "contents...", "/data/b.txt": "contents..."}
```

### write_file
Create or overwrite a file. Parent directories are created automatically.

```
write_file(path: "/data/output.txt", content: "hello")
→ {"path": "/data/output.txt", "bytes_written": 5}
```

### append_file
Append content to an existing file (creates if missing).

### list_directory
List files and directories with metadata (name, type, size).

```
list_directory(path: "/data")
→ [{"name": "file.txt", "type": "file", "size": 1024}, {"name": "subdir", "type": "directory"}]
```

### directory_tree
Recursive directory tree as JSON. Default depth 3.

```
directory_tree(path: "/data", max_depth: 2)
```

### get_file_info
Detailed metadata: size, type, permissions, symlink target.

### create_directory
Create a directory and any missing parents.

### move_file
Move or rename a file or directory.

### copy_file
Copy a file to a new location.

### delete_file
Delete a file. Destructive.

### delete_directory
Delete a directory. Use `recursive: true` for non-empty directories. Destructive.

### search_files
Search for files by glob pattern. Default depth 10.

```
search_files(path: "/data", pattern: "*.json", max_depth: 5)
→ ["/data/config.json", "/data/sub/data.json"]
```

### exists
Check if a path exists, and whether it's a file or directory.

## Workflow

1. `list_directory` or `directory_tree` to explore
2. `read_file` to inspect content
3. `write_file` / `append_file` to create or modify
4. `search_files` to find files by pattern
