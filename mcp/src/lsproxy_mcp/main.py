from typing import List
from mcp.server import Server
from mcp.types import TextContent
from .tools.definitions import TOOLS
from .tools.handlers import *

server = Server(name="lsproxy-mcp")

@server.list_tools()
async def handle_list_tools():
    return TOOLS

@server.call_tool()
async def handle_call_tool(name: str, arguments: dict) -> List[TextContent]:
    handler = {
        "definitions_in_file": handle_definitions_in_file,
        "find_definition": handle_find_definition,
        "find_references": handle_find_references,
        "list_files": handle_list_files,
        "read_source_code": handle_read_source_code
    }.get(name)

    if not handler:
        return [TextContent(type="text", text=f"Tool {name} not found")]

    return await handler(arguments)
