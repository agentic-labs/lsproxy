from typing import List, Dict, Any
from mcp.types import TextContent

def handle_error(error: Exception) -> List[TextContent]:
    return [TextContent(
        type="text",
        text=f"Error: {str(error)}"
    )]

def validate_required_fields(arguments: Dict[str, Any], required_fields: List[str]) -> List[TextContent]:
    missing_fields = [field for field in required_fields if field not in arguments]
    if missing_fields:
        return [TextContent(
            type="text",
            text=f"Error: Missing required arguments: {', '.join(missing_fields)}"
        )]
    return []

def validate_field_type(field_name: str, value: Any, expected_type: type) -> List[TextContent]:
    if not isinstance(value, expected_type):
        return [TextContent(
            type="text",
            text=f"Error: Field '{field_name}' must be of type {expected_type.__name__}"
        )]
    return []
