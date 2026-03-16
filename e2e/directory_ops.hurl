# Create directory
POST {{baseurl}}/tools/create-directory
{"arguments": {"path": "{{test_dir}}/newdir"}}
HTTP 200

# Check it exists
POST {{baseurl}}/tools/exists
{"arguments": {"path": "{{test_dir}}/newdir"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"is_directory\":true"

# Delete it
POST {{baseurl}}/tools/delete-directory
{"arguments": {"path": "{{test_dir}}/newdir"}}
HTTP 200
