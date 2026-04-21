# Write initial content
POST {{baseurl}}/tools/write_file
{"arguments": {"path": "{{test_dir}}/append.txt", "content": "hello"}}
HTTP 200

# Append to it
POST {{baseurl}}/tools/append_file
{"arguments": {"path": "{{test_dir}}/append.txt", "content": " world"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "bytes_appended"

# Verify
POST {{baseurl}}/tools/read_file
{"arguments": {"path": "{{test_dir}}/append.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "hello world"
