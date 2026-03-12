# Check existing file
POST {{host}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/exists_test.txt", "content": "hi"}}
HTTP 200

POST {{host}}/tools/exists
{"arguments": {"path": "{{test_dir}}/exists_test.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"exists\":true"
jsonpath "$.content[0].data" contains "\"is_file\":true"

# Check non-existent path
POST {{host}}/tools/exists
{"arguments": {"path": "{{test_dir}}/nonexistent.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"exists\":false"

# Check directory
POST {{host}}/tools/create-directory
{"arguments": {"path": "{{test_dir}}/exists_dir"}}
HTTP 200

POST {{host}}/tools/exists
{"arguments": {"path": "{{test_dir}}/exists_dir"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"exists\":true"
jsonpath "$.content[0].data" contains "\"is_directory\":true"
