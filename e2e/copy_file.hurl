# Setup: write a file
POST {{host}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/copy_src.txt", "content": "copy me"}}
HTTP 200

# Copy it
POST {{host}}/tools/copy-file
{"arguments": {"source": "{{test_dir}}/copy_src.txt", "destination": "{{test_dir}}/copy_dst.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "copy_src.txt"
jsonpath "$.content[0].data" contains "copy_dst.txt"
jsonpath "$.content[0].data" contains "bytes_copied"

# Verify both exist
POST {{host}}/tools/read-file
{"arguments": {"path": "{{test_dir}}/copy_dst.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "copy me"

POST {{host}}/tools/read-file
{"arguments": {"path": "{{test_dir}}/copy_src.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "copy me"
