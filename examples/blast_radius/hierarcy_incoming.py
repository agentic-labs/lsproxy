from typing import List

from pydantic import BaseModel
from client import APIClient
from models import (
    CodeContext,
    DefinitionResponse,
    FilePosition,
    FileSymbolsRequest,
    GetReferencesRequest,
)
import logging


def number_source_code(code_context: CodeContext) -> str:
    lines = code_context.source_code.splitlines()
    start_line = code_context.range.start.line
    unfenced = "\n".join(f"{i+start_line}| {line}" for i, line in enumerate(lines))
    return f"```\n{unfenced}\n```"


def to_prompt(code_context: CodeContext) -> str:
    result = f"## {code_context.range.path}\n\n"
    source_code = number_source_code(code_context)
    result += f"{source_code}\n\n"
    return result


class HierarchyItem(BaseModel):
    name: str
    kind: str
    defined_at: FilePosition
    source_code_context: CodeContext

    def __repr__(self) -> str:
        return f"{self.defined_at.path.rsplit('/')[-1]}:{self.defined_at.position.line}#{self.name}"

    def __str__(self) -> str:
        return self.__repr__()

    def __hash__(self) -> int:
        return hash(
            (
                self.defined_at.path,
                self.defined_at.position.line,
                self.defined_at.position.character,
            )
        )


def get_symbols_containing_position(
    client: APIClient, target_positions: List[FilePosition]
) -> List[HierarchyItem]:
    """
    Get symbols that are affected by changes to specific lines in a file.

    Args:
        client: APIClient instance to communicate with language server
        file_path: Path to the file being analyzed
        affected_lines: List of line numbers that were modified

    Returns:
        List of Symbol objects that are defined in or overlap with the affected lines
    """

    assert all(
        position.path == target_positions[0].path for position in target_positions
    ), "All positions must be in the same file"
    request = FileSymbolsRequest(
        file_path=target_positions[0].path, include_source_code=True
    )

    workspace_files = client.list_files()
    if request.file_path not in workspace_files:
        logging.error(f"File {request.file_path} not found in workspace")
        return []
    response = client.get_definitions_in_file(request)

    symbols_to_return = set()
    if response.source_code_context == None:
        return []
    for symbol, source_code_context in sorted(zip(
        response.symbols, response.source_code_context
    ), key=lambda x: x[1].range):
        unnamed = symbol.name == "<unknown>" or not symbol.name
        starts_after_affected_lines = all(
            symbol.start_position > target_position
            for target_position in target_positions
        )
        if unnamed or starts_after_affected_lines:
            #logging.info(f"Skipping symbol: {symbol.name} {symbol.kind}")
            continue

        for target_position in list(target_positions):
            if source_code_context.range.contains(target_position):
                symbols_to_return.add(
                    HierarchyItem(
                        name=symbol.name,
                        kind=symbol.kind,
                        defined_at=symbol.start_position,
                        source_code_context=source_code_context,
                    )
                )
                
    return list(symbols_to_return)


def get_hierarchy_incoming(
    client: APIClient, starting_positions: List[FilePosition]
) -> List[DefinitionResponse]:
    """ """
    symbol_stack = get_symbols_containing_position(client, starting_positions)
    edges = set()
    nodes = set()
    workspace_files = client.list_files()

    while symbol_stack:
        symbol = symbol_stack.pop()

        if symbol in nodes:
            continue
        nodes.add(symbol)

        try:
            references_response = client.find_references(
                GetReferencesRequest(
                    start_position=symbol.defined_at, include_declaration=False
                )
            )
        except Exception as e:
            logging.error(f"Error finding references for symbol {symbol}: {e}")
            continue

        # Group references by file for more efficient processing
        references_by_file = {}
        for ref in references_response.references:
            if ref.path not in workspace_files:
                logging.warning(f"File {ref.path} not found in workspace")
                references_by_file = {}
                break
            references_by_file.setdefault(ref.path, []).append(ref)

        logging.info(
            f"Found references in {len(references_by_file)} files for symbol {symbol.name}"
        )

        symbols_containing_references = []
        for references in references_by_file.values():
            symbols_containing_references.extend(
                get_symbols_containing_position(client, references)
            )

        for ref_symbol in symbols_containing_references:
            if ref_symbol in nodes:
                continue
            edges.add((symbol, ref_symbol))
            symbol_stack.append(ref_symbol)

    return nodes, edges
