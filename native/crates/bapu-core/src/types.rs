//! Data model mirroring the original TypeScript types (src/types/index.ts), so collections,
//! environments, and database connections saved by the Electron app can be read by this
//! native rewrite without a migration step.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::DELETE => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default, rename = "isSecret", skip_serializing_if = "Option::is_none")]
    pub is_secret: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAssertionResult {
    pub name: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyType {
    None,
    Json,
    Form,
    Raw,
    Graphql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    None,
    Bearer,
    Basic,
    Apikey,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, rename = "apiKeyName", skip_serializing_if = "Option::is_none")]
    pub api_key_name: Option<String>,
    #[serde(default, rename = "apiKeyValue", skip_serializing_if = "Option::is_none")]
    pub api_key_value: Option<String>,
    #[serde(default, rename = "addTo", skip_serializing_if = "Option::is_none")]
    pub add_to: Option<String>, // "header" | "query"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub params: Vec<KeyValuePair>,
    #[serde(default)]
    pub headers: Vec<KeyValuePair>,
    #[serde(rename = "bodyType")]
    pub body_type: BodyType,
    #[serde(rename = "bodyContent", default)]
    pub body_content: String,
    #[serde(rename = "graphqlQuery", default, skip_serializing_if = "Option::is_none")]
    pub graphql_query: Option<String>,
    #[serde(rename = "graphqlVariables", default, skip_serializing_if = "Option::is_none")]
    pub graphql_variables: Option<String>,
    #[serde(rename = "authType")]
    pub auth_type: AuthType,
    #[serde(rename = "authConfig", default)]
    pub auth_config: AuthConfig,
    #[serde(rename = "preRequestScript", default, skip_serializing_if = "Option::is_none")]
    pub pre_request_script: Option<String>,
    #[serde(rename = "testScript", default, skip_serializing_if = "Option::is_none")]
    pub test_script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<String>,
    #[serde(rename = "collectionId", default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
}

impl ApiRequest {
    /// True if this request has a pre-request or test script that would need to run
    /// through the sandboxed script engine. Used to warn on import, same as the Electron app.
    pub fn has_executable_script(&self) -> bool {
        self.pre_request_script.as_deref().is_some_and(|s| !s.trim().is_empty())
            || self.test_script.as_deref().is_some_and(|s| !s.trim().is_empty())
            || self.tests.as_deref().is_some_and(|s| !s.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    #[serde(rename = "statusText")]
    pub status_text: String,
    #[serde(rename = "timeMs")]
    pub time_ms: u64,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub headers: HashMap<String, String>,
    pub data: serde_json::Value,
    pub timestamp: String,
    #[serde(rename = "testResults", default, skip_serializing_if = "Vec::is_empty")]
    pub test_results: Vec<TestAssertionResult>,
    #[serde(rename = "consoleLogs", default, skip_serializing_if = "Vec::is_empty")]
    pub console_logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    #[serde(rename = "isPrimaryKey")]
    pub is_primary_key: bool,
    #[serde(rename = "isNullable")]
    pub is_nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    #[serde(rename = "rowCount")]
    pub row_count: u64,
    pub columns: Vec<ColumnDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbKind {
    Sqlite,
    Postgres,
    Mysql,
    Redis,
    Mongodb,
}

/// A database connection's configuration. Fields holding secrets (password, ssh_password,
/// ssh_passphrase, ssh_private_key) are never written to disk in plaintext — see
/// `crate::secrets`, which this struct's storage layer routes them through.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnection {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: DbKind,
    #[serde(rename = "connectionString", default, skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    pub database: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub ssl: bool,
    #[serde(rename = "sslCaCert", default, skip_serializing_if = "Option::is_none")]
    pub ssl_ca_cert: Option<String>,
    #[serde(rename = "sslClientCert", default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_cert: Option<String>,
    #[serde(rename = "sslClientKey", default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_key: Option<String>,
    /// Certificate validation is ON by default; this is an explicit, named opt-out,
    /// never a silent default (see the TLS fix in the Electron app's SECURITY.md).
    #[serde(rename = "sslRejectUnauthorized", default, skip_serializing_if = "Option::is_none")]
    pub ssl_reject_unauthorized: Option<bool>,
    #[serde(rename = "sshEnabled", default)]
    pub ssh_enabled: bool,
    #[serde(rename = "sshHost", default, skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    #[serde(rename = "sshPort", default, skip_serializing_if = "Option::is_none")]
    pub ssh_port: Option<String>,
    #[serde(rename = "sshUsername", default, skip_serializing_if = "Option::is_none")]
    pub ssh_username: Option<String>,
    #[serde(rename = "sshPassword", default, skip_serializing_if = "Option::is_none")]
    pub ssh_password: Option<String>,
    #[serde(rename = "sshPrivateKey", default, skip_serializing_if = "Option::is_none")]
    pub ssh_private_key: Option<String>,
    #[serde(rename = "sshPassphrase", default, skip_serializing_if = "Option::is_none")]
    pub ssh_passphrase: Option<String>,
    #[serde(rename = "isConnected", default)]
    pub is_connected: bool,
    #[serde(rename = "isDemoDb", default)]
    pub is_demo_db: bool,
    #[serde(default)]
    pub tables: Vec<TableSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlScriptTab {
    pub id: String,
    #[serde(rename = "databaseId")]
    pub database_id: String,
    pub name: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    #[serde(rename = "rowCount")]
    pub row_count: u64,
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<String>,
    #[serde(rename = "queryPlan", default, skip_serializing_if = "Option::is_none")]
    pub query_plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<KeyValuePair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String, // "api" | "sql"
    pub title: String,
    pub subtitle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub requests: Vec<ApiRequest>,
    /// When set, this collection's requests are mirrored to plain JSON files under this
    /// folder (one file per request, git-diffable) — same model as the Electron app.
    #[serde(rename = "folderPath", default, skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,
}
