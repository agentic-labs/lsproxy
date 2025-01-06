use std::collections::HashMap;
use once_cell::sync::Lazy;

use super::LanguageCallHierarchy;

trait HandlerFactory: Send + Sync {
    fn create(&self) -> Box<dyn LanguageCallHierarchy>;
}

struct PythonHandlerFactory;
impl HandlerFactory for PythonHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::python::PythonCallHierarchy {})
    }
}

struct TypeScriptHandlerFactory;
impl HandlerFactory for TypeScriptHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::typescript::TypeScriptCallHierarchy {})
    }
}

struct RustHandlerFactory;
impl HandlerFactory for RustHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::rust::RustCallHierarchy {})
    }
}

struct GoHandlerFactory;
impl HandlerFactory for GoHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::go::GoCallHierarchy {})
    }
}

struct JavaHandlerFactory;
impl HandlerFactory for JavaHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::java::JavaCallHierarchy {})
    }
}

struct CppHandlerFactory;
impl HandlerFactory for CppHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::cpp::CppCallHierarchy {})
    }
}

struct PhpHandlerFactory;
impl HandlerFactory for PhpHandlerFactory {
    fn create(&self) -> Box<dyn LanguageCallHierarchy> {
        Box::new(super::php::PhpCallHierarchy {})
    }
}

static HANDLERS: Lazy<HashMap<&str, Box<dyn HandlerFactory>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("python", Box::new(PythonHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("typescript", Box::new(TypeScriptHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("typescriptreact", Box::new(TypeScriptHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("javascript", Box::new(TypeScriptHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("typescriptjavascript", Box::new(TypeScriptHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("rust", Box::new(RustHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("golang", Box::new(GoHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("java", Box::new(JavaHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("cpp", Box::new(CppHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("c++", Box::new(CppHandlerFactory) as Box<dyn HandlerFactory>);
    m.insert("php", Box::new(PhpHandlerFactory) as Box<dyn HandlerFactory>);
    m
});

pub fn get_handler(language: &str) -> Option<Box<dyn LanguageCallHierarchy>> {
    HANDLERS.get(language).map(|factory| factory.create())
}