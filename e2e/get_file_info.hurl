POST {{host}}/tools/write-file
{"arguments": {"path": "{{test_dir}}/info.txt", "content": "hello"}}
HTTP 200

POST {{host}}/tools/get-file-info
{"arguments": {"path": "{{test_dir}}/info.txt"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "\"type\":\"file\""
jsonpath "$.content[0].data" contains "\"size\":5"
