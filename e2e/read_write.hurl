# Write a file
POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/hello.txt", "content": "Hello, ACT!"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"bytes_written\":11"

# Read it back
POST {{baseurl}}/tools/read-file
{"arguments": {"path": "{{test_dir}}/hello.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "Hello, ACT!"

# Read non-existent
POST {{baseurl}}/tools/read-file
{"arguments": {"path": "{{test_dir}}/nonexistent.txt"}}
HTTP *
[Asserts]
jsonpath "$.error.kind" == "std:not-found"
