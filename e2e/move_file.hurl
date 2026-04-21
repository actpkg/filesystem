# Setup
POST {{baseurl}}/tools/write_file
{"arguments": {"path": "{{test_dir}}/src.txt", "content": "moveme"}}
HTTP 200

# Move
POST {{baseurl}}/tools/move_file
{"arguments": {"source": "{{test_dir}}/src.txt", "destination": "{{test_dir}}/dst.txt"}}
HTTP 200

# Verify moved
POST {{baseurl}}/tools/read_file
{"arguments": {"path": "{{test_dir}}/dst.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "moveme"

# Original should be gone
POST {{baseurl}}/tools/read_file
{"arguments": {"path": "{{test_dir}}/src.txt"}}
HTTP *
[Asserts]
jsonpath "$.error.kind" == "std:not-found"
