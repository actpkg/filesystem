POST {{baseurl}}/tools
HTTP 200
[Asserts]
jsonpath "$.tools" count == 9
jsonpath "$.tools[*].name" "read_file"
jsonpath "$.tools[*].name" "write_file"
jsonpath "$.tools[*].name" "list_directory"
jsonpath "$.tools[*].name" "copy_file"
jsonpath "$.tools[*].name" "delete_file"
