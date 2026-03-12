POST {{host}}/tools
HTTP 200
[Asserts]
jsonpath "$.tools[*].name" includes "read-file"
jsonpath "$.tools[*].name" includes "write-file"
jsonpath "$.tools[*].name" includes "list-directory"
jsonpath "$.tools[*].name" includes "search-files"
jsonpath "$.tools[*].name" includes "exists"
