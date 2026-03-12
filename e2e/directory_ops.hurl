# Create directory
POST {{host}}/tools/create-directory
{"arguments": {"path": "{{test_dir}}/newdir"}}
HTTP 200

# Check it exists
POST {{host}}/tools/exists
{"arguments": {"path": "{{test_dir}}/newdir"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"is_directory\":true"

# Delete it
POST {{host}}/tools/delete-directory
{"arguments": {"path": "{{test_dir}}/newdir"}}
HTTP 200
