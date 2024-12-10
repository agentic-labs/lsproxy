use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast::{channel, Receiver, Sender};

pub trait JsonRpc: Send + Sync {
    fn create_request(&self, method: &str, params: Option<Value>) -> (u64, String);
    fn create_notification(&self, method: &str, params: Value) -> String;
    fn parse_message(&self, data: &str) -> Result<JsonRpcMessage, JsonRpcError>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InnerMessage {
    pub message: String,
    pub r#type: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

#[derive(Clone)]
pub struct JsonRpcHandler {
    id_counter: Arc<AtomicU64>,
}

impl JsonRpcHandler {
    pub fn new() -> Self {
        Self {
            id_counter: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl JsonRpc for JsonRpcHandler {
    fn create_request(&self, method: &str, params: Option<Value>) -> (u64, String) {
        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(serde_json::Value::Null)
        })
        .to_string();
        (id, request)
    }

    fn create_notification(&self, method: &str, params: Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        })
        .to_string()
    }

    fn parse_message(&self, data: &str) -> Result<JsonRpcMessage, JsonRpcError> {
        serde_json::from_str(data).map_err(|e| JsonRpcError {
            code: -32700,
            message: e.to_string(),
            data: None,
        })
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ExpectedMessageKey {
    pub method: String,
    pub params: Value,
}

#[derive(Clone)]
pub struct PendingRequests {
    request_channels: Arc<DashMap<u64, Sender<JsonRpcMessage>>>,
    notification_channels: Arc<DashMap<ExpectedMessageKey, Sender<JsonRpcMessage>>>,
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            request_channels: Arc::new(DashMap::new()),
            notification_channels: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_request(
        &self,
        id: u64,
    ) -> Result<Receiver<JsonRpcMessage>, Box<dyn Error + Send + Sync>> {
        let (tx, rx) = channel(16);
        self.request_channels.insert(id, tx);
        Ok(rx)
    }

    pub async fn remove_request(&self, id: u64) -> Option<Sender<JsonRpcMessage>> {
        self.request_channels.remove(&id).map(|(_, sender)| sender)
    }

    pub async fn add_notification(
        &self,
        key: ExpectedMessageKey,
    ) -> Result<Receiver<JsonRpcMessage>, Box<dyn Error + Send + Sync>> {
        let (tx, rx) = channel(16);
        self.notification_channels.insert(key, tx);
        Ok(rx)
    }

    pub async fn remove_notification(
        &self,
        key: &ExpectedMessageKey,
    ) -> Option<Sender<JsonRpcMessage>> {
        self.notification_channels
            .remove(key)
            .map(|(_, sender)| sender)
    }
}
