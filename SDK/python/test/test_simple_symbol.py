# coding: utf-8

"""
    lsproxy

    No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)

    The version of the OpenAPI document: 0.1.0
    Generated by OpenAPI Generator (https://openapi-generator.tech)

    Do not edit the class manually.
"""  # noqa: E501


import unittest

from openapi_client.models.simple_symbol import SimpleSymbol

class TestSimpleSymbol(unittest.TestCase):
    """SimpleSymbol unit test stubs"""

    def setUp(self):
        pass

    def tearDown(self):
        pass

    def make_instance(self, include_optional) -> SimpleSymbol:
        """Test SimpleSymbol
            include_optional is a boolean, when False only required
            params are included, when True both required and
            optional params are included """
        # uncomment below to create an instance of `SimpleSymbol`
        """
        model = SimpleSymbol()
        if include_optional:
            return SimpleSymbol(
                kind = '',
                location = openapi_client.models.simple_location.SimpleLocation(
                    identifier_start_character = 0, 
                    identifier_start_line = 0, 
                    uri = '', ),
                name = ''
            )
        else:
            return SimpleSymbol(
                kind = '',
                location = openapi_client.models.simple_location.SimpleLocation(
                    identifier_start_character = 0, 
                    identifier_start_line = 0, 
                    uri = '', ),
                name = '',
        )
        """

    def testSimpleSymbol(self):
        """Test SimpleSymbol"""
        # inst_req_only = self.make_instance(include_optional=False)
        # inst_req_and_optional = self.make_instance(include_optional=True)

if __name__ == '__main__':
    unittest.main()