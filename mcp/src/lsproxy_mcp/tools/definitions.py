from mcp.types import Tool

TOOLS = [
    Tool(
        name="definitions_in_file",
        description="Get all symbol definitions in a file",
        inputSchema={
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Path to the file"}
            },
            "required": ["file_path"]
        }
    ),
    Tool(
        name="find_definition",
        description="Find definition of a symbol",
        inputSchema={
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Path to the file containing the symbol"},
                "line": {"type": "integer", "description": "Line number of the symbol"},
                "character": {"type": "integer", "description": "Character position of the symbol"}
            },
            "required": ["file_path", "line", "character"]
        }
    ),
    Tool(
        name="find_references",
        description="Find all references to a symbol",
        inputSchema={
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Path to the file containing the symbol"},
                "line": {"type": "integer", "description": "Line number of the symbol"},
                "character": {"type": "integer", "description": "Character position of the symbol"}
            },
            "required": ["file_path", "line", "character"]
        }
    ),
    Tool(
        name="list_files",
        description="List all files in the workspace",
        inputSchema={
            "type": "object",
            "properties": {
                "glob": {"type": "string", "description": "Optional glob pattern to filter files"}
            }
        }
    ),
    Tool(
        name="read_source_code",
        description="Read source code from a file",
        inputSchema={
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Path to the file"},
                "start_line": {"type": "integer", "description": "Optional start line number"},
                "end_line": {"type": "integer", "description": "Optional end line number"}
            },
            "required": ["file_path"]
        }
    )
]
