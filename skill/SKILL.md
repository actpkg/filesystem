---
name: filesystem
description: File system operations — read, write, list, copy, move, delete
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

### write_file
Create or overwrite a file. Parent directories are created automatically.

```
write_file(path: "/data/output.txt", content: "hello")
→ {"path": "/data/output.txt", "bytes_written": 5}
```

### append_file
Append to a file without reading it first. Creates if missing. Saves tokens for large files.

```
append_file(path: "/data/log.txt", content: "new line\n")
```

### list_directory
List files and directories. Supports glob filtering and recursive search.

```
list_directory(path: "/data")
list_directory(path: "/data", glob: "*.rs", recursive: true)
list_directory(path: "/data", glob: "*.json", recursive: true, max_depth: 5)
```

Returns JSON array with name, type, size for each entry.

### move_file
Move or rename a file or directory.

### copy_file
Copy a file (filesystem-optimized).

### delete_file
Delete a file. Destructive.

### delete_directory
Delete a directory. Use `recursive: true` for non-empty. Destructive.

## Workflow

1. `list_directory` to explore (add `glob` + `recursive` to search)
2. `read_file` to inspect content
3. `write_file` / `append_file` to create or modify
