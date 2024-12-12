from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
import asyncio

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

            # Test list_files tool
            print("\nTesting list_files tool...")
            result = await session.call_tool("list_files", arguments={})
            print(f"list_files result: {result}")

if __name__ == "__main__":
    asyncio.run(run())
