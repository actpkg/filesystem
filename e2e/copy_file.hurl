# Setup: write a file
POST {{baseurl}}/tools/write_file
{"arguments": {"path": "{{test_dir}}/copy_src.txt", "content": "copy me"}}
HTTP 200

# Copy it
POST {{baseurl}}/tools/copy_file
{"arguments": {"source": "{{test_dir}}/copy_src.txt", "destination": "{{test_dir}}/copy_dst.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data.from" contains "copy_src.txt"
jsonpath "$.content[0].data.to" contains "copy_dst.txt"
jsonpath "$.content[0].data.bytes_copied" == 7

# Verify both exist
POST {{baseurl}}/tools/read_file
{"arguments": {"path": "{{test_dir}}/copy_dst.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "copy me"

POST {{baseurl}}/tools/read_file
{"arguments": {"path": "{{test_dir}}/copy_src.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "copy me"
