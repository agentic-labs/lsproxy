from typing import Dict, Any, List
import httpx
from mcp.types import TextContent
from ..config import get_settings
from .error_handler import handle_error, validate_required_fields, validate_field_type
import json

async def call_lsproxy(endpoint: str, method: str = "GET", params: Dict[str, Any] = None, json_data: Dict[str, Any] = None) -> Dict[str, Any]:
    settings = get_settings()
    try:
        async with httpx.AsyncClient() as client:
            response = await client.request(
                method,
                f"{settings.lsproxy_url}{endpoint}",
                params=params,
                json=json_data
            )
            if response.status_code >= 400:
                error_text = await response.text()
                raise Exception(f"LSProxy error ({response.status_code}): {error_text}")
            return response.json()
    except httpx.HTTPError as e:
        raise Exception(f"HTTP error: {str(e)}")

async def handle_definitions_in_file(arguments: Dict[str, Any]) -> List[TextContent]:
    error_response = validate_required_fields(arguments, ["file_path"])
    if error_response:
        return error_response

    try:
        settings = get_settings()
        result = await call_lsproxy(
            settings.endpoints["definitions_in_file"],
            params={"file_path": arguments["file_path"]}
        )
        return [TextContent(
            type="text",
            text=json.dumps(result, indent=2)
        )]
    except Exception as e:
        return handle_error(e)

async def handle_find_definition(arguments: Dict[str, Any]) -> List[TextContent]:
    error_response = validate_required_fields(arguments, ["position"])
    if error_response:
        return error_response

    try:
        settings = get_settings()
        result = await call_lsproxy(
            settings.endpoints["find_definition"],
            method="POST",
            json_data={
                "position": arguments["position"],
                "include_raw_response": arguments.get("include_raw_response", False),
                "include_source_code": arguments.get("include_source_code", False)
            }
        )
        return [TextContent(
            type="text",
            text=json.dumps(result, indent=2)
        )]
    except Exception as e:
        return handle_error(e)

async def handle_find_references(arguments: Dict[str, Any]) -> List[TextContent]:
    error_response = validate_required_fields(arguments, ["identifier_position"])
    if error_response:
        return error_response

    try:
        settings = get_settings()
        result = await call_lsproxy(
            settings.endpoints["find_references"],
            method="POST",
            json_data={
                "identifier_position": arguments["identifier_position"],
                "include_code_context_lines": arguments.get("include_code_context_lines"),
                "include_raw_response": arguments.get("include_raw_response", False)
            }
        )
        return [TextContent(
            type="text",
            text=json.dumps(result, indent=2)
        )]
    except Exception as e:
        return handle_error(e)

async def handle_list_files(arguments: Dict[str, Any]) -> List[TextContent]:
    try:
        settings = get_settings()
        result = await call_lsproxy(
            settings.endpoints["list_files"]
        )
        return [TextContent(
            type="text",
            text=json.dumps(result, indent=2)
        )]
    except Exception as e:
        return handle_error(e)

async def handle_read_source_code(arguments: Dict[str, Any]) -> List[TextContent]:
    error_response = validate_required_fields(arguments, ["path", "start", "end"])
    if error_response:
        return error_response

    try:
        settings = get_settings()
        result = await call_lsproxy(
            settings.endpoints["read_source_code"],
            method="POST",
            json_data={
                "path": arguments["path"],
                "start": arguments["start"],
                "end": arguments["end"]
            }
        )
        return [TextContent(
            type="text",
            text=result["source_code"]
        )]
    except Exception as e:
        return handle_error(e)

HANDLERS = {
    "definitions_in_file": handle_definitions_in_file,
    "find_definition": handle_find_definition,
    "find_references": handle_find_references,
    "list_files": handle_list_files,
    "read_source_code": handle_read_source_code
}
