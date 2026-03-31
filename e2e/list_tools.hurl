POST {{baseurl}}/tools
HTTP 200
[Asserts]
jsonpath "$.tools" count == 9
jsonpath "$.tools[*].name" contains "read-file"
jsonpath "$.tools[*].name" contains "write-file"
jsonpath "$.tools[*].name" contains "list-directory"
jsonpath "$.tools[*].name" contains "copy-file"
jsonpath "$.tools[*].name" contains "delete-file"
