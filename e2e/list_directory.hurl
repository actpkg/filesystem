# Setup: write files
POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/listdir/a.txt", "content": "a"}}
HTTP 200

POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/listdir/b.txt", "content": "b"}}
HTTP 200

# List
POST {{baseurl}}/tools/list-directory
{"arguments": {"path": "{{test_dir}}/listdir"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "a.txt"
jsonpath "$.content[0].data" contains "b.txt"
