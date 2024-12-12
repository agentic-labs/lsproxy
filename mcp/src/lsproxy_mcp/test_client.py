from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
import asyncio
import json

async def test_definitions_in_file(session):
    print("\nTesting definitions_in_file tool...")

    # Test 1: Valid input with existing file
    print("\nTest 1: Valid input with existing file")
    try:
        result = await session.call_tool(
            "definitions_in_file",
            {
                "file_path": "test/test.py"
            }
        )
        print("Valid input result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 2: Missing required field
    print("\nTest 2: Missing required field")
    try:
        result = await session.call_tool(
            "definitions_in_file",
            {}
        )
        print("Missing field result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 3: Invalid file path
    print("\nTest 3: Invalid file path")
    try:
        result = await session.call_tool(
            "definitions_in_file",
            {
                "file_path": "nonexistent/file.py"
            }
        )
        print("Invalid path result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

async def test_find_definition(session):
    print("\nTesting find_definition tool...")

    # Test 1: Valid input
    print("\nTest 1: Valid input")
    try:
        result = await session.call_tool(
            "find_definition",
            {
                "position": {
                    "path": "test/test.py",
                    "position": {
                        "line": 0,
                        "character": 5
                    }
                }
            }
        )
        print("Valid input result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 2: Missing required field
    print("\nTest 2: Missing required field")
    try:
        result = await session.call_tool(
            "find_definition",
            {}
        )
        print("Missing field result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 3: Invalid position format
    print("\nTest 3: Invalid position format")
    try:
        result = await session.call_tool(
            "find_definition",
            {
                "position": {
                    "path": "test/test.py",
                    "position": {
                        "line": "invalid",  # Should be integer
                        "character": 5
                    }
                }
            }
        )
        print("Invalid format result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

async def test_find_references(session):
    print("\nTesting find_references tool...")

    # Test 1: Valid input
    print("\nTest 1: Valid input")
    try:
        result = await session.call_tool(
            "find_references",
            {
                "identifier_position": {
                    "path": "test/test.py",
                    "position": {
                        "line": 0,
                        "character": 5
                    }
                },
                "include_code_context_lines": 2,
                "include_raw_response": False
            }
        )
        print("Valid input result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 2: Missing required field
    print("\nTest 2: Missing required field")
    try:
        result = await session.call_tool(
            "find_references",
            {}
        )
        print("Missing field result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 3: Invalid position format
    print("\nTest 3: Invalid position format")
    try:
        result = await session.call_tool(
            "find_references",
            {
                "identifier_position": {
                    "path": "test/test.py",
                    "position": {
                        "line": "invalid",  # Should be integer
                        "character": 5
                    }
                }
            }
        )
        print("Invalid format result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

async def test_list_files(session):
    print("\nTesting list_files tool...")

    # Test 1: Valid input with glob pattern
    print("\nTest 1: Valid input with glob pattern")
    try:
        result = await session.call_tool(
            "list_files",
            {
                "glob": "**/*.py"
            }
        )
        print("Valid input result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 2: No glob pattern (should list all files)
    print("\nTest 2: No glob pattern")
    try:
        result = await session.call_tool(
            "list_files",
            {}
        )
        print("No glob pattern result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 3: Invalid glob pattern
    print("\nTest 3: Invalid glob pattern")
    try:
        result = await session.call_tool(
            "list_files",
            {
                "glob": "[invalid"  # Invalid glob syntax
            }
        )
        print("Invalid glob pattern result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

async def test_read_source_code(session):
    print("\nTesting read_source_code tool...")

    # Test 1: Valid input
    print("\nTest 1: Valid input")
    try:
        result = await session.call_tool(
            "read_source_code",
            {
                "path": "test/test.py",
                "start": {
                    "line": 0,
                    "character": 0
                },
                "end": {
                    "line": 1,
                    "character": 0
                }
            }
        )
        print("Valid input result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 2: Missing required fields
    print("\nTest 2: Missing required fields")
    try:
        result = await session.call_tool(
            "read_source_code",
            {
                "path": "test/test.py"
                # Missing start and end
            }
        )
        print("Missing fields result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

    # Test 3: Invalid position format
    print("\nTest 3: Invalid position format")
    try:
        result = await session.call_tool(
            "read_source_code",
            {
                "path": "test/test.py",
                "start": {
                    "line": "invalid",  # Should be integer
                    "character": 0
                },
                "end": {
                    "line": 1,
                    "character": 0
                }
            }
        )
        print("Invalid format result:")
        for content in result.content:
            print(f"Type: {content.type}")
            print(f"Text: {content.text}")
    except Exception as e:
        print(f"Error: {str(e)}")

async def run():
    # Create server parameters for stdio connection
    server_params = StdioServerParameters(
        command="python",
        args=["-m", "lsproxy_mcp"],
        env=None
    )

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            # Initialize the connection
            await session.initialize()

            # List available tools
            print("Listing tools...")
            tools = await session.list_tools()
            print(f"Available tools: {tools}")

            # Test all tools
            await test_definitions_in_file(session)
            await test_find_definition(session)
            await test_find_references(session)
            await test_list_files(session)
            await test_read_source_code(session)

if __name__ == "__main__":
    asyncio.run(run())
