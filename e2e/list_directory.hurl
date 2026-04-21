# Setup: write files
POST {{baseurl}}/tools/write_file
{"arguments": {"path": "{{test_dir}}/listdir/a.txt", "content": "a"}}
HTTP 200

POST {{baseurl}}/tools/write_file
{"arguments": {"path": "{{test_dir}}/listdir/b.txt", "content": "b"}}
HTTP 200

# List
POST {{baseurl}}/tools/list_directory
{"arguments": {"path": "{{test_dir}}/listdir"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data[*].name" contains "a.txt"
jsonpath "$.content[0].data[*].name" contains "b.txt"
