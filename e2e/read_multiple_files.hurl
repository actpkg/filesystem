# Create files
POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/multi_a.txt", "content": "alpha"}}
HTTP 200

POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/multi_b.txt", "content": "beta"}}
HTTP 200

# Read multiple
POST {{baseurl}}/tools/read-multiple-files
{"arguments": {"paths": ["{{test_dir}}/multi_a.txt", "{{test_dir}}/multi_b.txt"]}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "alpha"
jsonpath "$.content[0].data" contains "beta"
