import httpx
import logging
from typing import List
from models import (
    SymbolResponse,
    DefinitionResponse,
    ReferencesResponse,
    GetDefinitionRequest,
    GetReferencesRequest,
    FileSymbolsRequest,
)
from tenacity import retry, stop_after_attempt, wait_exponential


logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class APIClient:
    """Client for interacting with the lsproxy API."""

        # Shared HTTP client with connection pooling
    _client = httpx.Client(
        base_url="http://localhost:4444/v1",
        timeout=30.0,
        headers={"Content-Type": "application/json"},
        limits=httpx.Limits(
            max_keepalive_connections=20,
            max_connections=100
        )
    )

    def __init__(self, base_url: str = "http://localhost:4444/v1", timeout: float = 30.0):
        self._client.base_url = base_url
        self._client.timeout = timeout


    def _request(self, method: str, endpoint: str, **kwargs) -> httpx.Response:
        logger.debug(f"{method} {endpoint} with params: {kwargs}")
        response = self._client.request(method, endpoint, **kwargs)
        logger.debug(f"Response Status: {response.status_code}")
        logger.debug(f"Response Body: {response.text}")
        response.raise_for_status()
        return response

    def get_definitions_in_file(self, request: FileSymbolsRequest) -> SymbolResponse:
        """Retrieve symbols from a specific file."""
        response = self._request(
            "GET", "/symbol/definitions-in-file", params=request.model_dump()
        )
        symbols = SymbolResponse.model_validate_json(response.text)
        logger.debug(
            f"Retrieved {len(symbols.symbols)} symbols from {request.file_path}"
        )
        return symbols

    def find_definition(self, request: GetDefinitionRequest) -> DefinitionResponse:
        """Get the definition of a symbol at a specific position in a file."""
        response = self._request(
            "POST", "/symbol/find-definition", json=request.model_dump()
        )
        definition = DefinitionResponse.model_validate_json(response.text)
        logger.debug(f"Found {len(definition.definitions)} definitions")
        return definition

    def find_references(self, request: GetReferencesRequest) -> ReferencesResponse:
        """Find all references to a symbol."""
        response = self._request(
            "POST", "/symbol/find-references", json=request.model_dump()
        )
        references = ReferencesResponse.model_validate_json(response.text)
        logger.debug(f"Found {len(references.references)} references")
        return references

    def list_files(self) -> List[str]:
        """Get a list of all files in the workspace."""
        response = self._request("GET", "/workspace/list-files")
        files = response.json()
        logger.debug(f"Retrieved {len(files)} files from workspace")
        return files

    def close(self):
        """Close the HTTP client."""
        self.client.close()
        logger.debug("HTTP client closed")
