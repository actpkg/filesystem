# Create a file
POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/to_delete.txt", "content": "delete me"}}
HTTP 200

# Delete it
POST {{baseurl}}/tools/delete-file
{"arguments": {"path": "{{test_dir}}/to_delete.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "to_delete.txt"

# Verify gone
POST {{baseurl}}/tools/read-file
{"arguments": {"path": "{{test_dir}}/to_delete.txt"}}
HTTP *
[Asserts]
jsonpath "$.error.kind" == "std:not-found"
