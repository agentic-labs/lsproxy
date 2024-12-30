pub trait LanguageCallHierarchy {
    fn get_function_call_query(&self) -> &'static str;
    fn get_function_definition_query(&self) -> &'static str;
    fn is_function_type(&self, node_type: &str) -> bool;
}

pub mod python;
pub mod typescript;
mod registry;

pub fn get_call_hierarchy_handler(language: &str) -> Option<Box<dyn LanguageCallHierarchy>> {
    registry::get_handler(language)
}