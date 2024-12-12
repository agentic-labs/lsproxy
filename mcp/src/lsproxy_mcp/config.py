from functools import lru_cache
from pydantic_settings import BaseSettings
from mcp.server.models import InitializationOptions

class Settings(BaseSettings):
    lsproxy_url: str = "http://localhost:4444"
    initialization_options: InitializationOptions = InitializationOptions(
        server_name="lsproxy-mcp",
        server_version="0.1.0",
        capabilities={"tools": {}}
    )

@lru_cache()
def get_settings() -> Settings:
    return Settings()
