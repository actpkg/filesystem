# Create a file
POST {{host}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/to_delete.txt", "content": "delete me"}}
HTTP 200

# Delete it
POST {{host}}/tools/delete-file
{"arguments": {"path": "{{test_dir}}/to_delete.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "to_delete.txt"

# Verify gone
POST {{host}}/tools/exists
{"arguments": {"path": "{{test_dir}}/to_delete.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"exists\":false"
