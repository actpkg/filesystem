POST {{baseurl}}/tools
HTTP 200
[Asserts]
jsonpath "$.tools[*].name" contains "read-file"
jsonpath "$.tools[*].name" contains "write-file"
jsonpath "$.tools[*].name" contains "list-directory"
jsonpath "$.tools[*].name" contains "search-files"
jsonpath "$.tools[*].name" contains "exists"
