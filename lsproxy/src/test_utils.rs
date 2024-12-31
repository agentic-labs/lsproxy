use crate::api_types::{set_thread_local_mount_dir, unset_thread_local_mount_dir};
use crate::lsp::manager::Manager;

pub fn python_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/python".to_string()
}

pub fn js_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/js".to_string()
}

pub fn java_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/java".to_string()
}

pub fn rust_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/rust".to_string()
}

pub fn go_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/go".to_string()
}

pub fn typescript_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/typescript".to_string()
}

pub fn cpp_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/cpp".to_string()
}

pub fn c_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/c".to_string()
}

pub fn php_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/php".to_string()
}

pub struct TestContext {
    pub manager: Option<Manager>,
}

impl TestContext {
    pub async fn setup(file_path: &str, manager: bool) -> Result<Self, Box<dyn std::error::Error>> {
        set_thread_local_mount_dir(file_path);
        if manager {
            let mut manager = Manager::new(file_path).await?;
            if let Err(e) = manager.start_langservers(file_path).await {
                unset_thread_local_mount_dir();
                return Err(e);
            }
            return Ok(Self {
                manager: Some(manager),
            });
        }
        Ok(Self { manager: None })
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        unset_thread_local_mount_dir();
    }
}
