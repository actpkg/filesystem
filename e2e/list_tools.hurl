POST {{baseurl}}/tools
HTTP 200
[Asserts]
jsonpath "$.tools" count == 9
jsonpath "$.tools[*].name" contains "read_file"
jsonpath "$.tools[*].name" contains "write_file"
jsonpath "$.tools[*].name" contains "list_directory"
jsonpath "$.tools[*].name" contains "copy_file"
jsonpath "$.tools[*].name" contains "delete_file"
