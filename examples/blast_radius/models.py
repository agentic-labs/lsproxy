import logging
from typing import List, Optional
from pydantic import BaseModel, Field
from enum import Enum


class SupportedLanguages(str, Enum):
    python = "python"
    typescript_javascript = "typescript_javascript"
    rust = "rust"


logger = logging.getLogger(__name__)


class Position(BaseModel):
    """Specific position within a file."""

    line: int = Field(..., description="0-indexed line number.", example=10, ge=0)
    character: int = Field(
        ..., description="0-indexed character index.", example=5, ge=0
    )

    def __lt__(self, other: "Position") -> bool:
        """Compare positions by line first, then character."""
        if self.line != other.line:
            return self.line < other.line
        return self.character < other.character

    def __eq__(self, other: "Position") -> bool:
        """Check if two positions are equal."""
        return self.line == other.line and self.character == other.character

    def __le__(self, other: "Position") -> bool:
        """Less than or equal comparison."""
        return self < other or self == other

    def __gt__(self, other: "Position") -> bool:
        """Greater than comparison."""
        return not (self <= other)

    def __ge__(self, other: "Position") -> bool:
        """Greater than or equal comparison."""
        return not (self < other)


class FilePosition(BaseModel):
    """Specific position within a file, including the file path."""

    path: str = Field(..., description="The path to the file.", example="src/main.py")
    position: Position = Field(..., description="The position within the file.")

    @property
    def as_tuple(self) -> tuple[str, int, int]:
        return (self.path, self.position.line, self.position.character)

    def __lt__(self, other: "FilePosition") -> bool:
        """Compare file positions by path first, then position."""
        if self.path != other.path:
            logger.warning(
                f"Comparing file positions with different paths: {self.path} and {other.path}"
            )
            return self.path < other.path
        return self.position < other.position

    def __eq__(self, other: "FilePosition") -> bool:
        """Check if two file positions are equal."""
        return self.path == other.path and self.position == other.position

    def __le__(self, other: "FilePosition") -> bool:
        """Less than or equal comparison."""
        return self < other or self == other

    def __gt__(self, other: "FilePosition") -> bool:
        """Greater than comparison."""
        return not (self <= other)

    def __ge__(self, other: "FilePosition") -> bool:
        """Greater than or equal comparison."""
        return not (self < other)


class FileRange(BaseModel):
    """Range within a file, defined by start and end positions."""

    path: str = Field(..., description="The path to the file.", example="src/main.py")
    start: Position = Field(..., description="Start position of the range.")
    end: Position = Field(..., description="End position of the range.")

    def contains(self, file_position: FilePosition) -> bool:
        """Check if a position is within the range."""
        return (
            self.path == file_position.path
            and self.start <= file_position.position
            and file_position.position <= self.end
        )
    
    def __lt__(self, other: "FileRange") -> bool:
        """Compare ranges by path first, then start position."""
        if self.path != other.path:
            return self.path < other.path
        return self.start < other.start
    
    def __eq__(self, other: "FileRange") -> bool:
        """Check if two ranges are equal."""
        return self.path == other.path and self.start == other.start and self.end == other.end
    
    def __le__(self, other: "FileRange") -> bool:
        """Less than or equal comparison."""
        return self < other or self == other

    def __gt__(self, other: "FileRange") -> bool:
        """Greater than comparison."""
        return not (self <= other)

    def __ge__(self, other: "FileRange") -> bool:
        """Greater than or equal comparison."""
        return not (self < other)


class CodeContext(BaseModel):
    """Contextual information of the source code around a symbol or reference."""

    range: FileRange = Field(..., description="The range within the file.")
    source_code: str = Field(
        ..., description="The source code within the specified range."
    )


class Symbol(BaseModel):
    """Representation of a symbol defined in the codebase."""

    kind: str = Field(
        ...,
        description="The kind of the symbol (e.g., function, class).",
        example="class",
    )
    name: str = Field(..., description="The name of the symbol.", example="User")
    start_position: FilePosition = Field(
        ..., description="The starting position of the symbol's identifier."
    )


class SymbolResponse(BaseModel):
    """Response containing a list of symbols."""

    symbols: List[Symbol] = Field(
        ..., description="List of symbols found in the specified file."
    )
    raw_response: Optional[dict] = Field(
        None,
        description=(
            "The raw response from the language server.\n\n"
            "https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol\n"
            "https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#document_symbol"
        ),
    )

    source_code_context: Optional[List[CodeContext]] = Field(
        None, description="Full source code of each symbol definition."
    )


class DefinitionResponse(BaseModel):
    """Response containing definition locations of a symbol."""

    definitions: List[FilePosition] = Field(
        ..., description="List of definition locations for the symbol."
    )
    raw_response: Optional[dict] = Field(
        None,
        description=(
            "The raw response from the language server.\n\n"
            "https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition"
        ),
    )
    source_code_context: Optional[List[CodeContext]] = Field(
        None, description="Source code contexts of the symbol definitions."
    )


class GetDefinitionRequest(BaseModel):
    """Request to retrieve the definition of a symbol."""

    position: FilePosition = Field(
        ...,
        description="The position of the symbol whose definition is to be retrieved.",
    )
    include_raw_response: Optional[bool] = Field(
        False,
        description="Whether to include the raw response from the language server.",
    )
    include_source_code: Optional[bool] = Field(
        False,
        description="Whether to include the source code around the symbol's identifier.",
    )


class ReferencesResponse(BaseModel):
    """Response containing references to a symbol."""

    references: List[FilePosition] = Field(
        ..., description="List of reference locations for the symbol."
    )
    context: Optional[List[CodeContext]] = Field(
        None, description="Source code contexts around the references."
    )
    raw_response: Optional[dict] = Field(
        None,
        description=(
            "The raw response from the language server.\n\n"
            "https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references"
        ),
    )


class GetReferencesRequest(BaseModel):
    """Request to find all references to a symbol."""

    start_position: FilePosition = Field(
        ...,
        description="The starting position of the symbol whose references are to be found.",
    )
    include_code_context_lines: Optional[int] = Field(
        None,
        description="Number of source code lines to include around each reference.",
        ge=0,
    )
    include_declaration: Optional[bool] = Field(
        False,
        description="Whether to include the declaration of the symbol in the references.",
    )
    include_raw_response: Optional[bool] = Field(
        False,
        description="Whether to include the raw response from the language server.",
    )


class FileSymbolsRequest(BaseModel):
    """Request to retrieve symbols from a specific file."""

    file_path: str = Field(
        ...,
        description="The path to the file to get the symbols for, relative to the root of the workspace.",
        example="src/main.py",
    )
    include_raw_response: Optional[bool] = Field(
        False,
        description="Whether to include the raw response from the language server.",
    )

    include_source_code: Optional[bool] = Field(
        False,
        description="Whether to include the source code of each symbol definition.",
    )


class ErrorResponse(BaseModel):
    """Response representing an error."""

    error: str = Field(..., description="The error message.")
