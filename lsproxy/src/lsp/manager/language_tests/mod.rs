use lsp_types::{GotoDefinitionResponse, Location, Range, Url};
use tokio::time::{sleep, Duration};

use crate::test_utils::{
    c_sample_path, cpp_sample_path, go_sample_path, java_sample_path, js_sample_path,
    php_sample_path, python_sample_path, rust_sample_path, typescript_sample_path, TestContext,
};

use crate::api_types::{FilePosition, FileRange, Position, Symbol, SymbolResponse};

mod c_tests;
mod cpp_tests;
mod golang_tests;
mod java_tests;
mod js_tests;
mod php_tests;
mod python_tests;
mod rust_tests;
mod tsx_tests;
mod typescript_tests;
