# coding: utf-8

"""
    lsproxy

    No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)

    The version of the OpenAPI document: 0.1.0
    Generated by OpenAPI Generator (https://openapi-generator.tech)

    Do not edit the class manually.
"""  # noqa: E501


import unittest

from lsproxy_sdk.api.workspace_api import WorkspaceApi


class TestWorkspaceApi(unittest.TestCase):
    """WorkspaceApi unit test stubs"""

    def setUp(self) -> None:
        self.api = WorkspaceApi()

    def tearDown(self) -> None:
        pass

    def test_workspace_files(self) -> None:
        """Test case for workspace_files

        Get a list of all files in the workspace
        """
        pass


if __name__ == '__main__':
    unittest.main()