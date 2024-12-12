from mcp.types import Tool

TOOLS = [
    Tool(
        name="definitions_in_file",
        description="Get all symbol definitions in a file (uses ast-grep)",
        inputSchema={
            "type": "object",
            "required": ["file_path"],
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The path to the file to get the symbols for, relative to the root of the workspace.",
                    "example": "src/main.py"
                }
            }
        }
    ),
    Tool(
        name="find_definition",
        description="Get the definition of a symbol at a specific position in a file",
        inputSchema={
            "type": "object",
            "required": ["position"],
            "properties": {
                "position": {
                    "type": "object",
                    "required": ["path", "position"],
                    "properties": {
                        "path": {
                            "type": "string",
                            "example": "src/main.py"
                        },
                        "position": {
                            "type": "object",
                            "required": ["line", "character"],
                            "properties": {
                                "line": {
                                    "type": "integer",
                                    "format": "int32",
                                    "description": "0-indexed line number",
                                    "minimum": 0,
                                    "example": 10
                                },
                                "character": {
                                    "type": "integer",
                                    "format": "int32",
                                    "description": "0-indexed character index",
                                    "minimum": 0,
                                    "example": 5
                                }
                            }
                        }
                    }
                },
                "include_raw_response": {
                    "type": "boolean",
                    "description": "Whether to include the raw response from the langserver in the response. Defaults to false.",
                    "example": False
                },
                "include_source_code": {
                    "type": "boolean",
                    "description": "Whether to include the source code around the symbol's identifier in the response. Defaults to false.",
                    "example": False
                }
            }
        }
    ),
    Tool(
        name="find_references",
        description="Find all references to a symbol",
        inputSchema={
            "type": "object",
            "required": ["identifier_position"],
            "properties": {
                "identifier_position": {
                    "type": "object",
                    "required": ["path", "position"],
                    "properties": {
                        "path": {
                            "type": "string",
                            "example": "src/main.py"
                        },
                        "position": {
                            "type": "object",
                            "required": ["line", "character"],
                            "properties": {
                                "line": {
                                    "type": "integer",
                                    "format": "int32",
                                    "description": "0-indexed line number",
                                    "minimum": 0,
                                    "example": 10
                                },
                                "character": {
                                    "type": "integer",
                                    "format": "int32",
                                    "description": "0-indexed character index",
                                    "minimum": 0,
                                    "example": 5
                                }
                            }
                        }
                    }
                },
                "include_code_context_lines": {
                    "type": ["integer", "null"],
                    "format": "int32",
                    "description": "Whether to include the source code of the symbol in the response. Defaults to none.",
                    "minimum": 0,
                    "example": 5
                },
                "include_raw_response": {
                    "type": "boolean",
                    "description": "Whether to include the raw response from the langserver in the response. Defaults to false.",
                    "example": False
                }
            }
        }
    ),
    Tool(
        name="list_files",
        description="Get a list of all files in the workspace",
        inputSchema={
            "type": "object",
            "properties": {
                "glob": {
                    "type": "string",
                    "description": "Optional glob pattern to filter files",
                    "example": "**/*.py"
                }
            }
        }
    ),
    Tool(
        name="read_source_code",
        description="Read source code from a file in the workspace",
        inputSchema={
            "type": "object",
            "required": ["path", "start", "end"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file.",
                    "example": "src/main.py"
                },
                "start": {
                    "type": "object",
                    "required": ["line", "character"],
                    "properties": {
                        "line": {
                            "type": "integer",
                            "format": "int32",
                            "description": "0-indexed line number",
                            "minimum": 0,
                            "example": 10
                        },
                        "character": {
                            "type": "integer",
                            "format": "int32",
                            "description": "0-indexed character index",
                            "minimum": 0,
                            "example": 5
                        }
                    },
                    "description": "The start position of the range."
                },
                "end": {
                    "type": "object",
                    "required": ["line", "character"],
                    "properties": {
                        "line": {
                            "type": "integer",
                            "format": "int32",
                            "description": "0-indexed line number",
                            "minimum": 0,
                            "example": 10
                        },
                        "character": {
                            "type": "integer",
                            "format": "int32",
                            "description": "0-indexed character index",
                            "minimum": 0,
                            "example": 5
                        }
                    },
                    "description": "The end position of the range."
                }
            }
        }
    )
]
