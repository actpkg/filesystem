# Setup
POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/search/foo.rs", "content": ""}}
HTTP 200

POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/search/bar.rs", "content": ""}}
HTTP 200

POST {{baseurl}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/search/baz.txt", "content": ""}}
HTTP 200

# Search for *.rs
POST {{baseurl}}/tools/search-files
{"arguments": {"path": "{{test_dir}}/search", "pattern": "*.rs"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "foo.rs"
jsonpath "$.content[0].data" contains "bar.rs"
