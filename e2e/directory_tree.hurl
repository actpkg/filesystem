# Create nested structure
POST {{host}}/tools/create-directory
{"arguments": {"path": "{{test_dir}}/tree/sub1"}}
HTTP 200

POST {{host}}/tools/create-directory
{"arguments": {"path": "{{test_dir}}/tree/sub2"}}
HTTP 200

POST {{host}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/tree/root.txt", "content": "root"}}
HTTP 200

POST {{host}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/tree/sub1/child.txt", "content": "child"}}
HTTP 200

# Get tree
POST {{host}}/tools/directory-tree
{"arguments": {"path": "{{test_dir}}/tree"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "sub1"
jsonpath "$.content[0].data" contains "sub2"
jsonpath "$.content[0].data" contains "root.txt"
jsonpath "$.content[0].data" contains "child.txt"
