from typing import Dict, Any, List
import httpx
from mcp.types import TextContent
from ..config import get_settings

async def call_lsproxy(endpoint: str, method: str = "GET", params: Dict[str, Any] = None, json: Dict[str, Any] = None) -> Dict[str, Any]:
    settings = get_settings()
    async with httpx.AsyncClient() as client:
        response = await client.request(
            method,
            f"{settings.lsproxy_url}/v1{endpoint}",
            params=params,
            json=json
        )
        response.raise_for_status()
        return response.json()

async def handle_definitions_in_file(arguments: Dict[str, Any]) -> List[TextContent]:
    try:
        result = await call_lsproxy(
            "/symbol/definitions-in-file",
            params={"file_path": arguments["file_path"]}
        )
        return [TextContent(type="text", text=str(result))]
    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]

async def handle_find_definition(arguments: Dict[str, Any]) -> List[TextContent]:
    try:
        result = await call_lsproxy(
            "/symbol/find-definition",
            method="POST",
            json={
                "position": arguments["position"],
                "include_raw_response": arguments.get("include_raw_response", False),
                "include_source_code": arguments.get("include_source_code", False)
            }
        )
        return [TextContent(type="text", text=str(result))]
    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]

async def handle_find_references(arguments: Dict[str, Any]) -> List[TextContent]:
    try:
        result = await call_lsproxy(
            "/symbol/find-references",
            method="POST",
            json={
                "identifier_position": arguments["identifier_position"],
                "include_code_context_lines": arguments.get("include_code_context_lines"),
                "include_raw_response": arguments.get("include_raw_response", False)
            }
        )
        return [TextContent(type="text", text=str(result))]
    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]

async def handle_list_files(arguments: Dict[str, Any]) -> List[TextContent]:
    try:
        result = await call_lsproxy("/workspace/list-files")
        return [TextContent(type="text", text=str(result))]
    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]

async def handle_read_source_code(arguments: Dict[str, Any]) -> List[TextContent]:
    try:
        result = await call_lsproxy(
            "/workspace/read-source-code",
            method="POST",
            json={
                "path": arguments["path"],
                "start": arguments["start"],
                "end": arguments["end"]
            }
        )
        return [TextContent(type="text", text=result["source_code"])]
    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]

HANDLERS = {
    "definitions_in_file": handle_definitions_in_file,
    "find_definition": handle_find_definition,
    "find_references": handle_find_references,
    "list_files": handle_list_files,
    "read_source_code": handle_read_source_code
}
