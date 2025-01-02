import lsproxy
import pytest

@pytest.fixture(scope="session")
def lsproxy_talkformai():
    # This should be fine to re-use between tests because it's stateless.
    return lsproxy.Lsproxy.initialize_with_modal(
        repo_url="https://github.com/nsbradford/talkformai",
        sha="8fc82a7c302a724ae606801390b6ae9975b5a72d",
        timeout=300,  # seconds
        version="0.2.4",
    )

def test_lsproxy_go_to_definition_tool__hard_typescript_types(
    lsproxy_talkformai: lsproxy.Lsproxy
):
    # This has some funky indirection - the GitHub UI can't find it, but my local VSCode can.
    # going to "forms.name"
    # https://github.com/nsbradford/TalkFormAI/blob/main/src/pages/forms/%5Bid%5D.tsx#L63
    # the Form type is defined in types.ts, in reference to the supabase.ts file
    # https://github.com/nsbradford/TalkFormAI/blob/main/src/types.ts#L16
    # the supabase.ts file has the root definition of the Form.name property
    # https://github.com/nsbradford/TalkFormAI/blob/main/types/supabase.ts#L9
    lsp = lsproxy_talkformai
    assert lsp.check_health() == True
    defs: lsproxy.DefinitionResponse = lsp.find_definition(
        lsproxy.GetDefinitionRequest(
            position=lsproxy.FilePosition(
                path="src/pages/forms/[id].tsx",
                position=lsproxy.Position(line=62, character=12),
            ),
            include_raw_response=False,
            include_source_code=False,
        )
    )
    assert defs.source_code_context



# def test_lsproxy_go_to_definition_tool__tsx_component_property(
#     lsproxy_talkformai: lsproxy.Lsproxy
# ):
#     # Finding "pageTitle" in `    <Page pageTitle={`${camelCaseTitle}`} user={user}>`
#     # https://github.com/nsbradford/TalkFormAI/blob/main/src/pages/forms/%5Bid%5D.tsx#L65
#     lsp = lsproxy_talkformai
#     assert lsp.check_health() == True
#     defs: lsproxy.DefinitionResponse = lsp.find_definition(
#         lsproxy.GetDefinitionRequest(
#             position=lsproxy.FilePosition(
#                 path="src/pages/forms/[id].tsx",
#                 position=lsproxy.Position(line=64, character=15),
#             ),
#             include_raw_response=False,
#             include_source_code=True,
#         )
#     )
#     assert defs.source_code_context


# def test_lsproxy_crashes(lsproxy_talkformai: lsproxy.Lsproxy):
#     lsp = lsproxy_talkformai
#     assert lsp.check_health() == True
#     lsp.find_definition(
#         lsproxy.GetDefinitionRequest(
#             position=lsproxy.FilePosition(
#                 path="src/pages/forms/[id].tsx",
#                 position=lsproxy.Position(line=5, character=4),
#             ),
#             include_raw_response=False,
#             include_source_code=True,
#         )
#     )
#     assert lsp.check_health() == True
#     # this second find_definition call crashes the server
#     lsproxy_talkformai.find_definition(
#         lsproxy.GetDefinitionRequest(
#             position=lsproxy.FilePosition(
#                 path="src/components/chat.tsx",
#                 position=lsproxy.Position(line=9, character=19),
#             ),
#             include_raw_response=False,
#             include_source_code=True,
#         )
#     )
#     assert lsp.check_health() == True
