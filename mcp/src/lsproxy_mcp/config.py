from functools import lru_cache
from pydantic_settings import BaseSettings
from mcp.server.models import InitializationOptions

class Settings(BaseSettings):
    lsproxy_url: str = "http://localhost:4444"
    endpoints: dict = {
        "definitions_in_file": "/v1/symbol/definitions-in-file",
        "find_definition": "/v1/symbol/find-definition",
        "find_references": "/v1/symbol/find-references",
        "list_files": "/v1/workspace/list-files",
        "read_source_code": "/v1/workspace/read-source-code"
    }
    initialization_options: InitializationOptions = InitializationOptions(
        server_name="lsproxy-mcp",
        server_version="0.1.0",
        capabilities={"tools": {}}
    )

@lru_cache()
def get_settings() -> Settings:
    return Settings()
