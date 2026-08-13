"""A refusal from the host is reported as a refusal, not as a crash.

The `client` fixture grants only this test's private directory. Anything
outside it is refused by the host, and the component must say so with
`std:capability-denied` — the kind an agent can act on (ask for access, try
another path). `std:internal` says "this component broke", which sends an
agent looking for a bug that is not there.

Before this was fixed, only `read_file` mapped a refusal; the other tools,
and listing a directory whose entries the host refuses, all answered
`std:internal`.
"""

DENIED = "std:capability-denied"


async def test_reading_outside_the_grant_is_denied(client, tmp_path, expect_error):
    outside = tmp_path.parent / f"{tmp_path.name}-outside.bin"
    await expect_error(client, "read_binary_file", {"path": str(outside)}, DENIED)


async def test_writing_outside_the_grant_is_denied(client, tmp_path, expect_error):
    outside = tmp_path.parent / f"{tmp_path.name}-outside.txt"
    await expect_error(
        client, "write_file", {"path": str(outside), "content": "x"}, DENIED
    )


async def test_appending_outside_the_grant_is_denied(client, tmp_path, expect_error):
    outside = tmp_path.parent / f"{tmp_path.name}-outside.log"
    await expect_error(
        client, "append_file", {"path": str(outside), "content": "x"}, DENIED
    )


async def test_listing_outside_the_grant_is_denied(client, tmp_path, expect_error):
    await expect_error(client, "list_directory", {"path": str(tmp_path.parent)}, DENIED)


async def test_copying_out_of_the_grant_is_denied(client, tmp_path, expect_error):
    inside = tmp_path / "a.txt"
    await client.call_tool("write_file", {"path": str(inside), "content": "a"})
    outside = tmp_path.parent / f"{tmp_path.name}-copy.txt"
    await expect_error(
        client,
        "copy_file",
        {"source": str(inside), "destination": str(outside)},
        DENIED,
    )


async def test_deleting_outside_the_grant_is_denied(client, tmp_path, expect_error):
    outside = tmp_path.parent / f"{tmp_path.name}-gone.txt"
    await expect_error(client, "delete_file", {"path": str(outside)}, DENIED)
