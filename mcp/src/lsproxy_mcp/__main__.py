import asyncio
from mcp.server.models import InitializationOptions
import mcp.server.stdio
from .main import server

async def run():
    # Run the server as STDIO
    async with mcp.server.stdio.stdio_server() as (read_stream, write_stream):
        await server.run(
            read_stream,
            write_stream,
            InitializationOptions(
                server_name="lsproxy-mcp",
                server_version="0.1.0",
                capabilities={"tools": {}}
            )
        )

if __name__ == "__main__":
    asyncio.run(run())
