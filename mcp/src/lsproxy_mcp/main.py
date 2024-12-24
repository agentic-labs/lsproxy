from typing import List
from mcp.server import Server
from mcp.types import TextContent
from .tools.definitions import TOOLS
from .tools.handlers import HANDLERS

server = Server(name="lsproxy-mcp")

@server.list_tools()
async def handle_list_tools():
    try:
        return TOOLS
    except Exception as e:
        return [TextContent(
            type="error",
            text=f"Error listing tools: {str(e)}"
        )]

@server.call_tool()
async def handle_call_tool(name: str, arguments: dict) -> List[TextContent]:
    try:
        handler = HANDLERS.get(name)
        if not handler:
            return [TextContent(
                type="error",
                text=f"Tool '{name}' not found"
            )]

        # Validate tool exists in definitions
        tool = next((t for t in TOOLS if t.name == name), None)
        if not tool:
            return [TextContent(
                type="error",
                text=f"Tool '{name}' is not properly configured"
            )]

        return await handler(arguments)
    except Exception as e:
        return [TextContent(
            type="error",
            text=f"Error executing tool '{name}': {str(e)}"
        )]
