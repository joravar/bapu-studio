use crate::ui;
use bapu_core::ai_copilot::{self, AiProvider, AiRequest};
use bapu_core::codegen::CodeLang;
use bapu_core::cookies::CookieJar;
use bapu_core::db_admin::ServerHealthReport;
use bapu_core::db_ddl::quote_dialect_ident;
use bapu_core::db_diff::{DataDiffReport, SchemaDiffReport};
use bapu_core::db_export_import::{ExportFormat, GridFilterOp};
use bapu_core::db_sql_tools::ScriptExecutionReport;
use bapu_core::mock_server::{self, MockHit, MockServerHandle};
use bapu_core::runner::{self, CollectionRunReport, RunnerOptions};
use bapu_core::script_engine::ScriptExtraContext;
use bapu_core::stream_client::{
    self, FrameDirection, StreamEvent, StreamFrame, StreamHandle, StreamKind,
};
use bapu_core::types::{
    ApiRequest, ApiResponse, AuthConfig, BatchMutationResult, BodyType, Collection,
    ColumnDefinition, CookieEntry, DatabaseConnection, DbCategory, DbKind, Environment,
    ForeignKeyDef, HistoryItem, HttpMethod, KeyValuePair, MutationOp, QueryResult, RowMutation,
    SqlScriptTab, SqlSnippet, TableSchema,
};
use bapu_core::Storage;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Handle;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Tab {
    Api,
    Database,
    Streams,
    Secrets,
    History,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ApiReqSubTab {
    Params,
    Headers,
    Body,
    Auth,
    PreScript,
    Tests,
    Examples,
    Settings,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ApiRespSubTab {
    Preview,
    Raw,
    Headers,
    Cookies,
    Tests,
    Console,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum DbSubView {
    Grid,
    RecordStats,
    StructureDdl,
    Erd,
    ConsoleLog,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CompareMode {
    Schema,
    Data,
}

pub enum AsyncEvent {
    ApiCompleted {
        result: Result<ApiResponse, String>,
        updated_env_vars: HashMap<String, String>,
        updated_globals: HashMap<String, String>,
        updated_col_vars: HashMap<String, String>,
        new_cookies: Vec<CookieEntry>,
        history_entry: HistoryItem,
    },
    OAuth2TokenFetched(Result<String, String>),
    RunnerCompleted {
        result: Result<CollectionRunReport, String>,
        cookies: Vec<CookieEntry>,
    },
    MockServerStarted(Result<MockServerHandle, String>),
    QueryResult {
        result: Result<QueryResult, String>,
        history_entry: HistoryItem,
    },
    ConnectionTest(Result<String, String>),
    SchemaLoaded {
        db_id: String,
        result: Result<Vec<TableSchema>, String>,
    },
    PlaygroundSeeded {
        db: DatabaseConnection,
    },
    BatchMutationCompleted(Result<BatchMutationResult, String>),
    SqlScriptCompleted {
        result: Result<ScriptExecutionReport, String>,
        history_entry: HistoryItem,
    },
    DdlExecuted(Result<(), String>),
    DataImportCompleted(Result<usize, String>),
    SchemaCompareCompleted(Result<SchemaDiffReport, String>),
    DataCompareCompleted(Result<DataDiffReport, String>),
    ServerHealthLoaded(Result<ServerHealthReport, String>),
    AiResponse(Result<String, String>),
}

pub struct BapuApp {
    pub storage: Storage,
    pub runtime: Handle,
    pub http_client: reqwest::Client,

    pub tab: Tab,
    pub dark_mode: bool,

    // Global Quick Find (Ctrl+K)
    pub quick_find_open: bool,
    pub quick_find_query: String,

    // API Studio
    pub collections: Vec<Collection>,
    pub collapsed_collections: HashSet<String>,
    pub new_collection_name: String,
    pub new_collection_open: bool,
    pub active_request: ApiRequest,
    pub api_req_subtab: ApiReqSubTab,
    pub api_resp_subtab: ApiRespSubTab,
    pub resp_filter: String,
    pub last_response: Option<ApiResponse>,
    pub api_busy: bool,
    pub api_error: Option<String>,
    pub curl_import_open: bool,
    pub curl_import_text: String,
    pub io_modal_open: bool,
    pub io_modal_text: String,
    pub io_modal_status: Option<String>,

    // Code Generator, Cookie Jar, Collection Runner & Local Mock Server
    pub codegen_open: bool,
    pub codegen_lang: CodeLang,
    pub cookie_modal_open: bool,
    pub cookie_jar: CookieJar,
    pub new_cookie_domain: String,
    pub new_cookie_name: String,
    pub new_cookie_value: String,

    pub runner_open: bool,
    pub runner_col_idx: usize,
    pub runner_iterations: usize,
    pub runner_delay_ms: u64,
    pub runner_data_text: String,
    pub runner_busy: bool,
    pub runner_error: Option<String>,
    pub runner_report: Option<CollectionRunReport>,

    pub mock_open: bool,
    pub mock_col_idx: usize,
    pub mock_port: String,
    pub mock_handle: Option<MockServerHandle>,
    pub mock_error: Option<String>,
    pub mock_hits: Vec<MockHit>,
    pub mock_hits_tx: mpsc::Sender<MockHit>,
    pub mock_hits_rx: mpsc::Receiver<MockHit>,

    // Database Studio
    pub databases: Vec<DatabaseConnection>,
    pub active_db_index: usize,
    pub db_config_open: bool,
    pub db_engine_picker_filter: String,
    pub db_engine_picker_cat: Option<DbCategory>,
    pub db_schema_filter: String,
    pub db_subview: DbSubView,
    pub sql_tabs: Vec<SqlScriptTab>,
    pub active_sql_tab: usize,
    pub sql_snippets: Vec<SqlSnippet>,
    pub sql_param_values: HashMap<String, String>,
    pub last_query_result: Option<Result<QueryResult, String>>,
    pub last_script_report: Option<ScriptExecutionReport>,
    pub db_busy: bool,
    pub db_status: Option<String>,
    pub db_export_format: ExportFormat,
    pub ddl_preview_dialect: Option<DbKind>,

    // Staged Transactional Data Grid + Filter / Sort / Pagination / Inspector
    pub grid_table_name: Option<String>,
    pub grid_pending_edits: HashMap<(usize, String), String>,
    pub grid_staged_inserts: Vec<HashMap<String, String>>,
    pub grid_staged_deletes: HashSet<usize>,
    pub grid_quick_filter: String,
    pub grid_filter_col: String,
    pub grid_filter_op: GridFilterOp,
    pub grid_filter_val: String,
    pub grid_sort_col: Option<String>,
    pub grid_sort_asc: bool,
    pub grid_page: usize,
    pub grid_page_size: usize,
    pub selected_grid_row: usize,

    // ER Diagram & DBeaver PRO Tool Modals
    pub erd_positions: HashMap<String, [f32; 2]>,
    pub erd_zoom: f32,
    pub table_designer_open: bool,
    pub designer_table_name: String,
    pub designer_columns: Vec<ColumnDefinition>,
    pub designer_fks: Vec<ForeignKeyDef>,

    pub db_import_open: bool,
    pub import_target_table: String,
    pub import_raw_text: String,
    pub import_source_cols: Vec<String>,
    pub import_col_mapping: HashMap<String, String>,
    pub import_status: Option<String>,

    pub db_compare_open: bool,
    pub compare_mode: CompareMode,
    pub compare_source_db_idx: usize,
    pub compare_target_db_idx: usize,
    pub compare_table_name: String,
    pub schema_diff_report: Option<SchemaDiffReport>,
    pub data_diff_report: Option<DataDiffReport>,

    pub db_mock_gen_open: bool,
    pub mock_gen_table: String,
    pub mock_gen_count: usize,

    pub db_admin_open: bool,
    pub server_health: Option<Result<ServerHealthReport, String>>,

    // Stream Studio
    pub stream_kind: StreamKind,
    pub stream_url: String,
    pub stream_connected: bool,
    pub stream_outbound: String,
    pub stream_frames: Vec<StreamFrame>,
    pub stream_error: Option<String>,
    pub stream_handle: Option<StreamHandle>,
    pub stream_events_tx: mpsc::Sender<StreamEvent>,
    pub stream_events_rx: mpsc::Receiver<StreamEvent>,

    // AI Copilot
    pub ai_modal_open: bool,
    pub ai_provider: AiProvider,
    pub ai_base_url: String,
    pub ai_model: String,
    pub ai_api_key: String,
    pub ai_prompt: String,
    pub ai_output: String,
    pub ai_busy: bool,
    pub ai_error: Option<String>,

    // Environments, Globals & Secrets
    pub environments: Vec<Environment>,
    pub active_env_index: usize,
    pub globals: Vec<KeyValuePair>,
    pub new_env_name: String,
    pub revealed_secrets: HashSet<String>,

    // History
    pub history: Vec<HistoryItem>,
    pub history_filter: String,

    pub events_tx: mpsc::Sender<AsyncEvent>,
    pub events_rx: mpsc::Receiver<AsyncEvent>,
}

pub fn blank_request() -> ApiRequest {
    ApiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        name: "New Request".into(),
        method: HttpMethod::GET,
        url: "https://jsonplaceholder.typicode.com/users/1".into(),
        headers: vec![KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: "Accept".into(),
            value: "application/json".into(),
            enabled: true,
            is_secret: None,
        }],
        test_script: Some(
            "bapu.test(\"Status is 200\", () => {\n  bapu.response.to.have.status(200);\n});".into(),
        ),
        ..Default::default()
    }
}

fn starter_collections() -> Vec<Collection> {
    let col_id = "col-starter".to_string();
    vec![Collection {
        id: col_id.clone(),
        name: "Quickstart APIs".into(),
        description: Some("Sample REST endpoints with assertions and dynamic variables".into()),
        variables: vec![],
        folder_path: None,
        requests: vec![
            ApiRequest {
                id: "req-users".into(),
                name: "List Sample Users".into(),
                method: HttpMethod::GET,
                url: "https://jsonplaceholder.typicode.com/users".into(),
                params: vec![KeyValuePair {
                    id: "p1".into(),
                    key: "_limit".into(),
                    value: "5".into(),
                    enabled: true,
                    is_secret: None,
                }],
                headers: vec![KeyValuePair {
                    id: "h1".into(),
                    key: "Accept".into(),
                    value: "application/json".into(),
                    enabled: true,
                    is_secret: None,
                }],
                test_script: Some(
                    "bapu.test(\"Returns 200 OK\", () => {\n  bapu.response.to.have.status(200);\n});"
                        .into(),
                ),
                collection_id: Some(col_id.clone()),
                ..Default::default()
            },
            ApiRequest {
                id: "req-post".into(),
                name: "Create User ({{$randomEmail}})".into(),
                method: HttpMethod::POST,
                url: "https://jsonplaceholder.typicode.com/users".into(),
                headers: vec![KeyValuePair {
                    id: "h2".into(),
                    key: "Content-Type".into(),
                    value: "application/json".into(),
                    enabled: true,
                    is_secret: None,
                }],
                body_type: BodyType::Json,
                body_content: "{\n  \"id\": \"{{$guid}}\",\n  \"name\": \"{{$randomFullName}}\",\n  \"email\": \"{{$randomEmail}}\"\n}".into(),
                test_script: Some(
                    "bapu.test(\"Created 201\", () => {\n  bapu.response.to.have.status(201);\n});"
                        .into(),
                ),
                collection_id: Some(col_id),
                ..Default::default()
            },
        ],
    }]
}

fn default_environment() -> Environment {
    Environment {
        id: "env-default".into(),
        name: "Local Dev".into(),
        variables: vec![
            KeyValuePair {
                id: "v-base".into(),
                key: "API_BASE_URL".into(),
                value: "https://jsonplaceholder.typicode.com".into(),
                enabled: true,
                is_secret: Some(false),
            },
            KeyValuePair {
                id: "v-token".into(),
                key: "API_SECRET_KEY".into(),
                value: "sk_dev_bapu_local_vault".into(),
                enabled: true,
                is_secret: Some(true),
            },
        ],
    }
}

fn default_sql_tab() -> SqlScriptTab {
    SqlScriptTab {
        id: uuid::Uuid::new_v4().to_string(),
        database_id: String::new(),
        name: "Query 1.sql".into(),
        query: "SELECT * FROM \"users\" LIMIT 25;".into(),
    }
}

impl BapuApp {
    pub fn new(cc: &eframe::CreationContext<'_>, runtime: Handle) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        ui::theme::apply_theme(&cc.egui_ctx, true);

        let storage = Storage::new().expect("failed to initialize the local data directory");

        let mut collections = storage.load_collections().unwrap_or_default();
        if collections.is_empty() && !storage.has_file("collections.json") {
            collections = starter_collections();
        }
        let databases = storage.load_databases().unwrap_or_default();
        let sql_snippets = storage.load_sql_snippets().unwrap_or_default();
        let mut sql_tabs = storage.load_sql_tabs().unwrap_or_default();
        if sql_tabs.is_empty() {
            sql_tabs.push(default_sql_tab());
        }
        let ui_prefs = storage.load_ui_prefs().unwrap_or_default();
        let history = storage.load_history().unwrap_or_default();
        let cookies = storage.load_cookies().unwrap_or_default();
        let globals = storage.load_globals().unwrap_or_default();
        let mut environments = storage.load_environments().unwrap_or_default();
        if environments.is_empty() {
            environments.push(default_environment());
        }

        let active_request = collections
            .first()
            .and_then(|c| c.requests.first().cloned())
            .unwrap_or_else(blank_request);

        let (events_tx, events_rx) = mpsc::channel();
        let (stream_events_tx, stream_events_rx) = mpsc::channel();
        let (mock_hits_tx, mock_hits_rx) = mpsc::channel();

        Self {
            storage,
            runtime,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build the HTTP client"),
            tab: Tab::Api,
            dark_mode: ui_prefs.dark_mode,
            quick_find_open: false,
            quick_find_query: String::new(),
            collections,
            collapsed_collections: HashSet::new(),
            new_collection_name: String::new(),
            new_collection_open: false,
            active_request,
            api_req_subtab: ApiReqSubTab::Params,
            api_resp_subtab: ApiRespSubTab::Preview,
            resp_filter: String::new(),
            last_response: None,
            api_busy: false,
            api_error: None,
            curl_import_open: false,
            curl_import_text: String::new(),
            io_modal_open: false,
            io_modal_text: String::new(),
            io_modal_status: None,
            codegen_open: false,
            codegen_lang: CodeLang::ShellCurl,
            cookie_modal_open: false,
            cookie_jar: CookieJar::from_entries(cookies),
            new_cookie_domain: "localhost".into(),
            new_cookie_name: String::new(),
            new_cookie_value: String::new(),
            runner_open: false,
            runner_col_idx: 0,
            runner_iterations: 1,
            runner_delay_ms: 0,
            runner_data_text: String::new(),
            runner_busy: false,
            runner_error: None,
            runner_report: None,
            mock_open: false,
            mock_col_idx: 0,
            mock_port: "4010".into(),
            mock_handle: None,
            mock_error: None,
            mock_hits: vec![],
            mock_hits_tx,
            mock_hits_rx,
            databases,
            active_db_index: 0,
            db_config_open: false,
            db_engine_picker_filter: String::new(),
            db_engine_picker_cat: None,
            db_schema_filter: String::new(),
            db_subview: DbSubView::Grid,
            sql_tabs,
            active_sql_tab: 0,
            sql_snippets,
            sql_param_values: HashMap::new(),
            last_query_result: None,
            last_script_report: None,
            db_busy: false,
            db_status: None,
            db_export_format: ExportFormat::Csv,
            ddl_preview_dialect: None,
            grid_table_name: None,
            grid_pending_edits: HashMap::new(),
            grid_staged_inserts: Vec::new(),
            grid_staged_deletes: HashSet::new(),
            grid_quick_filter: String::new(),
            grid_filter_col: String::new(),
            grid_filter_op: GridFilterOp::Contains,
            grid_filter_val: String::new(),
            grid_sort_col: None,
            grid_sort_asc: true,
            grid_page: 0,
            grid_page_size: 50,
            selected_grid_row: 0,
            erd_positions: HashMap::new(),
            erd_zoom: 1.0,
            table_designer_open: false,
            designer_table_name: "new_table".into(),
            designer_columns: vec![
                ColumnDefinition {
                    name: "id".into(),
                    col_type: "INTEGER".into(),
                    is_primary_key: true,
                    auto_increment: true,
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "name".into(),
                    col_type: "TEXT".into(),
                    is_nullable: false,
                    ..Default::default()
                },
            ],
            designer_fks: vec![],
            db_import_open: false,
            import_target_table: String::new(),
            import_raw_text: String::new(),
            import_source_cols: vec![],
            import_col_mapping: HashMap::new(),
            import_status: None,
            db_compare_open: false,
            compare_mode: CompareMode::Schema,
            compare_source_db_idx: 0,
            compare_target_db_idx: 0,
            compare_table_name: String::new(),
            schema_diff_report: None,
            data_diff_report: None,
            db_mock_gen_open: false,
            mock_gen_table: String::new(),
            mock_gen_count: 25,
            db_admin_open: false,
            server_health: None,
            stream_kind: StreamKind::WebSocket,
            stream_url: "wss://echo.websocket.events".into(),
            stream_connected: false,
            stream_outbound: r#"{"event": "ping", "from": "Bapu Studio"}"#.into(),
            stream_frames: vec![StreamFrame {
                direction: FrameDirection::System,
                payload: "Ready to connect to WebSocket or SSE endpoint.".into(),
                timestamp: "00:00:00".into(),
            }],
            stream_error: None,
            stream_handle: None,
            stream_events_tx,
            stream_events_rx,
            ai_modal_open: false,
            ai_provider: AiProvider::Ollama,
            ai_base_url: AiProvider::Ollama.default_base_url().into(),
            ai_model: AiProvider::Ollama.default_model().into(),
            ai_api_key: String::new(),
            ai_prompt: String::new(),
            ai_output: String::new(),
            ai_busy: false,
            ai_error: None,
            environments,
            active_env_index: 0,
            globals,
            new_env_name: String::new(),
            revealed_secrets: HashSet::new(),
            history,
            history_filter: String::new(),
            events_tx,
            events_rx,
        }
    }

    pub fn palette(&self) -> ui::theme::Palette {
        ui::theme::Palette::for_mode(self.dark_mode)
    }

    pub fn active_env(&self) -> Environment {
        self.environments
            .get(self.active_env_index)
            .cloned()
            .unwrap_or_else(default_environment)
    }

    pub fn active_sql_mut(&mut self) -> &mut String {
        if self.sql_tabs.is_empty() {
            self.sql_tabs.push(default_sql_tab());
            self.active_sql_tab = 0;
        }
        let idx = self.active_sql_tab.min(self.sql_tabs.len() - 1);
        &mut self.sql_tabs[idx].query
    }

    pub fn active_sql(&self) -> &str {
        self.sql_tabs
            .get(self.active_sql_tab)
            .map(|t| t.query.as_str())
            .unwrap_or("")
    }

    pub fn save_active_request_to_collection(&mut self, col_idx: usize) {
        if let Some(col) = self.collections.get_mut(col_idx) {
            let mut saved = self.active_request.clone();
            saved.collection_id = Some(col.id.clone());
            self.active_request.collection_id = Some(col.id.clone());
            if let Some(pos) = col.requests.iter().position(|r| r.id == saved.id) {
                col.requests[pos] = saved;
            } else {
                col.requests.push(saved);
            }
            if let Some(folder) = &col.folder_path {
                let _ = bapu_core::collection_sync::write_collection_folder(
                    std::path::Path::new(folder),
                    col,
                );
            }
            self.persist();
        }
    }

    pub fn delete_request_at(&mut self, col_idx: usize, req_idx: usize) {
        let mut deleted_id = None;
        if let Some(col) = self.collections.get_mut(col_idx) {
            if req_idx < col.requests.len() {
                let removed = col.requests.remove(req_idx);
                deleted_id = Some(removed.id);
                if let Some(folder) = &col.folder_path {
                    let _ = bapu_core::collection_sync::write_collection_folder(
                        std::path::Path::new(folder),
                        col,
                    );
                }
            }
        }
        if let Some(id) = deleted_id {
            if self.active_request.id == id {
                self.active_request = self
                    .collections
                    .get(col_idx)
                    .and_then(|c| c.requests.first().cloned())
                    .or_else(|| {
                        self.collections
                            .iter()
                            .find_map(|c| c.requests.first().cloned())
                    })
                    .unwrap_or_else(blank_request);
                self.last_response = None;
            }
            self.persist();
        }
    }

    pub fn delete_collection_at(&mut self, col_idx: usize) {
        if col_idx >= self.collections.len() {
            return;
        }
        let removed = self.collections.remove(col_idx);
        self.collapsed_collections.remove(&removed.id);
        if self.active_request.collection_id.as_deref() == Some(removed.id.as_str())
            || removed.requests.iter().any(|r| r.id == self.active_request.id)
        {
            self.active_request = self
                .collections
                .iter()
                .find_map(|c| c.requests.first().cloned())
                .unwrap_or_else(blank_request);
            self.last_response = None;
        }
        if self.runner_col_idx >= self.collections.len() {
            self.runner_col_idx = self.collections.len().saturating_sub(1);
        }
        if self.mock_col_idx >= self.collections.len() {
            self.mock_col_idx = self.collections.len().saturating_sub(1);
        }
        self.persist();
    }

    pub fn delete_database_at(&mut self, idx: usize) {
        if idx >= self.databases.len() {
            return;
        }
        let was_active = idx == self.active_db_index;
        let removed = self.databases.remove(idx);
        let _ = self.storage.delete_database_secrets(&removed.id);
        if self.databases.is_empty() {
            self.active_db_index = 0;
            self.db_config_open = false;
            self.grid_table_name = None;
            self.last_query_result = None;
            self.db_status = None;
        } else if self.active_db_index >= self.databases.len() {
            self.active_db_index = self.databases.len() - 1;
        } else if idx < self.active_db_index {
            self.active_db_index -= 1;
        }
        if was_active {
            self.grid_table_name = None;
            self.last_query_result = None;
            self.revert_grid_changes();
        }
        self.persist();
    }

    pub fn send_active_request(&mut self) {
        if self.active_request.url.trim().is_empty() || self.api_busy {
            return;
        }
        self.api_busy = true;
        self.api_error = None;

        let client = self.http_client.clone();
        let request = self.active_request.clone();
        let mut env = self.active_env();
        let globals = self.globals.clone();
        let col_vars = request
            .collection_id
            .as_ref()
            .and_then(|cid| self.collections.iter().find(|c| &c.id == cid))
            .map(|c| c.variables.clone())
            .unwrap_or_default();
        let mut jar = self.cookie_jar.clone();
        let tx = self.events_tx.clone();

        self.runtime.spawn(async move {
            let mut updated_env_vars = HashMap::new();
            let mut updated_globals = HashMap::new();
            let mut updated_col_vars = HashMap::new();
            let mut pre_logs = Vec::new();

            let extra = ScriptExtraContext {
                globals: globals.clone(),
                collection_vars: col_vars.clone(),
                iteration: 0,
            };

            if let Some(pre) = request.pre_request_script.as_deref().filter(|s| !s.trim().is_empty())
            {
                let pre_res = bapu_core::script_engine::execute_script_with_context(
                    pre, &request, &env, None, &extra,
                );
                pre_logs.extend(pre_res.logs);
                for (k, v) in pre_res.updated_env_vars {
                    updated_env_vars.insert(k.clone(), v.clone());
                    apply_kv(&mut env.variables, &k, &v);
                }
                updated_globals.extend(pre_res.updated_globals);
                updated_col_vars.extend(pre_res.updated_collection_vars);
            }

            let scoped = bapu_core::env_vars::build_scoped_vars(&globals, &col_vars, &env, None);
            let resolved_request =
                bapu_core::env_vars::interpolate_request_with_vars(&request, &scoped);
            let url_for_history = resolved_request.url.clone();
            let method_str = resolved_request.method.as_str().to_string();

            let mut result =
                bapu_core::http_client::send_request_with_cookies(&client, &resolved_request, Some(&mut jar))
                    .await
                    .map_err(|e| e.to_string());

            let status_code = result.as_ref().ok().map(|r| r.status);

            if let Ok(ref mut resp) = result {
                resp.console_logs.extend(pre_logs);
                let test_src = request
                    .test_script
                    .as_deref()
                    .or(request.tests.as_deref())
                    .unwrap_or("");
                if !test_src.trim().is_empty() {
                    let test_res = bapu_core::script_engine::execute_script_with_context(
                        test_src,
                        &resolved_request,
                        &env,
                        Some(resp),
                        &extra,
                    );
                    resp.test_results = test_res.test_results;
                    resp.console_logs.extend(test_res.logs);
                    updated_env_vars.extend(test_res.updated_env_vars);
                    updated_globals.extend(test_res.updated_globals);
                    updated_col_vars.extend(test_res.updated_collection_vars);
                }
            }

            let history_entry = HistoryItem {
                id: uuid::Uuid::new_v4().to_string(),
                kind: "api".into(),
                title: format!("{method_str} {url_for_history}"),
                subtitle: request.name.clone(),
                status: status_code,
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            let _ = tx.send(AsyncEvent::ApiCompleted {
                result,
                updated_env_vars,
                updated_globals,
                updated_col_vars,
                new_cookies: jar.entries,
                history_entry,
            });
        });
    }

    pub fn fetch_oauth2_for_active_request(&mut self) {
        if self.api_busy {
            return;
        }
        self.api_busy = true;
        self.api_error = None;
        let client = self.http_client.clone();
        let env = self.active_env();
        let resolved = bapu_core::env_vars::interpolate_request(&self.active_request, &env);
        let cfg: AuthConfig = resolved.auth_config;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let res = bapu_core::auth::fetch_oauth2_access_token(&client, &cfg)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::OAuth2TokenFetched(res));
        });
    }

    pub fn start_collection_run(&mut self) {
        if self.runner_busy {
            return;
        }
        let Some(col) = self.collections.get(self.runner_col_idx).cloned() else {
            self.runner_error = Some("No collection selected.".into());
            return;
        };
        let data_rows = match runner::parse_iteration_data(&self.runner_data_text) {
            Ok(rows) => rows,
            Err(e) => {
                self.runner_error = Some(format!("Data parse error: {e}"));
                return;
            }
        };
        self.runner_busy = true;
        self.runner_error = None;

        let client = self.http_client.clone();
        let env = self.active_env();
        let globals = self.globals.clone();
        let mut jar = self.cookie_jar.clone();
        let opts = RunnerOptions {
            iterations: self.runner_iterations.max(1),
            delay_ms: self.runner_delay_ms,
            data_rows,
        };
        let tx = self.events_tx.clone();

        self.runtime.spawn(async move {
            let res = runner::run_collection(&client, &col, &env, &globals, &mut jar, &opts)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::RunnerCompleted {
                result: res,
                cookies: jar.entries,
            });
        });
    }

    pub fn start_mock(&mut self) {
        self.stop_mock();
        let Some(col) = self.collections.get(self.mock_col_idx).cloned() else {
            self.mock_error = Some("Select a collection to mock.".into());
            return;
        };
        let port: u16 = self.mock_port.trim().parse().unwrap_or(4010);
        let env = self.active_env();
        let hits_tx = self.mock_hits_tx.clone();
        let tx = self.events_tx.clone();

        self.runtime.spawn(async move {
            let res = mock_server::start_mock_server(port, col, env, Some(hits_tx))
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::MockServerStarted(res));
        });
    }

    pub fn stop_mock(&mut self) {
        if let Some(mut h) = self.mock_handle.take() {
            h.stop();
        }
    }

    pub fn create_sqlite_playground(&mut self) {
        if self.db_busy {
            return;
        }
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let path = std::env::temp_dir().join("bapu_studio_playground.sqlite");
            let _ = std::fs::File::create(&path);
            let mut db = DatabaseConnection {
                id: format!("sqlite-playground-{}", uuid::Uuid::new_v4()),
                name: "SQLite Playground".into(),
                kind: DbKind::Sqlite,
                connection_string: None,
                host: None,
                port: None,
                database: path.to_string_lossy().into_owned(),
                username: None,
                password: None,
                ssl: false,
                ssl_ca_cert: None,
                ssl_client_cert: None,
                ssl_client_key: None,
                ssl_reject_unauthorized: None,
                ssh_enabled: false,
                ssh_host: None,
                ssh_port: None,
                ssh_username: None,
                ssh_password: None,
                ssh_private_key: None,
                ssh_passphrase: None,
                is_connected: true,
                is_demo_db: false,
                tables: vec![],
            };
            let driver = bapu_core::db::driver_for(DbKind::Sqlite);
            let _ = driver
                .query(
                    &db,
                    "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE, plan TEXT NOT NULL)",
                )
                .await;
            let _ = driver
                .query(
                    &db,
                    "CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL, amount_cents INTEGER NOT NULL, status TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE)",
                )
                .await;
            let _ = driver
                .query(
                    &db,
                    "CREATE INDEX IF NOT EXISTS idx_orders_user_id ON orders(user_id)",
                )
                .await;
            let _ = driver
                .query(
                    &db,
                    "CREATE VIEW IF NOT EXISTS enterprise_orders AS SELECT o.id, u.name AS customer, o.amount_cents, o.status FROM orders o JOIN users u ON u.id = o.user_id WHERE u.plan = 'Enterprise'",
                )
                .await;
            let count = driver
                .query(&db, "SELECT COUNT(*) AS n FROM users")
                .await
                .ok()
                .and_then(|r| r.rows.first()?.get("n")?.as_i64())
                .unwrap_or(0);
            if count == 0 {
                let _ = driver
                    .query(
                        &db,
                        "INSERT INTO users (name, email, plan) VALUES ('Ada Lovelace', 'ada@bapustudio.dev', 'Enterprise'), ('Grace Hopper', 'grace@bapustudio.dev', 'Pro'), ('Linus Torvalds', 'linus@kernel.org', 'OSS'), ('Margaret Hamilton', 'margaret@apollo.gov', 'Enterprise')",
                    )
                    .await;
                let _ = driver
                    .query(
                        &db,
                        "INSERT INTO orders (user_id, amount_cents, status) VALUES (1, 14900, 'paid'), (2, 4900, 'paid'), (4, 29900, 'processing')",
                    )
                    .await;
            }
            if let Ok(tables) = driver.get_schema(&db).await {
                db.tables = tables;
            }
            let _ = tx.send(AsyncEvent::PlaygroundSeeded { db });
        });
    }

    pub fn pending_grid_change_count(&self) -> usize {
        self.grid_pending_edits.len()
            + self.grid_staged_inserts.len()
            + self.grid_staged_deletes.len()
    }

    pub fn revert_grid_changes(&mut self) {
        self.grid_pending_edits.clear();
        self.grid_staged_inserts.clear();
        self.grid_staged_deletes.clear();
    }

    pub fn commit_grid_changes(&mut self) {
        if self.db_busy || self.pending_grid_change_count() == 0 {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        let Some(table) = self.grid_table_name.clone() else {
            self.db_status =
                Some("Select a table from Schema to enable transactional edits.".into());
            return;
        };
        let Some(Ok(ref qr)) = self.last_query_result else {
            return;
        };

        let pk_cols: Vec<String> = db
            .tables
            .iter()
            .find(|t| t.name == table)
            .map(|t| {
                t.columns
                    .iter()
                    .filter(|c| c.is_primary_key)
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                if qr.columns.iter().any(|c| c == "_id") {
                    vec!["_id".into()]
                } else if qr.columns.iter().any(|c| c == "id") {
                    vec!["id".into()]
                } else {
                    vec![]
                }
            });

        let mut mutations: Vec<RowMutation> = Vec::new();
        let mut updates_by_row: HashMap<usize, HashMap<String, serde_json::Value>> = HashMap::new();
        for ((row_idx, col), new_str) in &self.grid_pending_edits {
            if self.grid_staged_deletes.contains(row_idx) {
                continue;
            }
            updates_by_row
                .entry(*row_idx)
                .or_default()
                .insert(col.clone(), parse_cell_input(new_str));
        }

        for (row_idx, values) in updates_by_row {
            if pk_cols.is_empty() {
                self.db_status = Some("Cannot update rows: no primary key column found.".into());
                return;
            }
            if let Some(orig_row) = qr.rows.get(row_idx) {
                let mut where_clause = HashMap::new();
                for pk in &pk_cols {
                    if let Some(v) = orig_row.get(pk) {
                        where_clause.insert(pk.clone(), v.clone());
                    }
                }
                mutations.push(RowMutation {
                    op: MutationOp::Update,
                    values,
                    where_clause,
                });
            }
        }

        for row_idx in &self.grid_staged_deletes {
            if pk_cols.is_empty() {
                self.db_status = Some("Cannot delete rows: no primary key column found.".into());
                return;
            }
            if let Some(orig_row) = qr.rows.get(*row_idx) {
                let mut where_clause = HashMap::new();
                for pk in &pk_cols {
                    if let Some(v) = orig_row.get(pk) {
                        where_clause.insert(pk.clone(), v.clone());
                    }
                }
                mutations.push(RowMutation {
                    op: MutationOp::Delete,
                    values: HashMap::new(),
                    where_clause,
                });
            }
        }

        for new_row in &self.grid_staged_inserts {
            let mut values = HashMap::new();
            for (k, v) in new_row {
                if !v.trim().is_empty() {
                    values.insert(k.clone(), parse_cell_input(v));
                }
            }
            if !values.is_empty() {
                mutations.push(RowMutation {
                    op: MutationOp::Insert,
                    values,
                    where_clause: HashMap::new(),
                });
            }
        }

        if mutations.is_empty() {
            self.revert_grid_changes();
            return;
        }

        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let res = driver
                .mutate_batch(&db, &table, &mutations)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::BatchMutationCompleted(res));
        });
    }

    pub fn run_sql_text(&mut self, raw_sql: String) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            self.last_query_result = Some(Err("No database connection selected.".into()));
            return;
        };
        let sql = bapu_core::db_sql_tools::bind_sql_params(&raw_sql, &self.sql_param_values);
        self.revert_grid_changes();
        self.selected_grid_row = 0;
        self.grid_table_name = detect_single_table(&sql);
        self.db_busy = true;
        let db_name = db.name.clone();
        let sql_preview: String = sql.lines().next().unwrap_or("").chars().take(80).collect();
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let result = driver.query(&db, &sql).await.map_err(|e| e.to_string());
            let history_entry = HistoryItem {
                id: uuid::Uuid::new_v4().to_string(),
                kind: "sql".into(),
                title: sql_preview,
                subtitle: db_name,
                status: Some(if result.is_ok() { 200 } else { 500 }),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };
            let _ = tx.send(AsyncEvent::QueryResult {
                result,
                history_entry,
            });
        });
    }

    pub fn select_and_query_table(&mut self, table_name: &str) {
        let Some(kind) = self.databases.get(self.active_db_index).map(|d| d.kind) else {
            return;
        };
        let qt = quote_dialect_ident(table_name, kind);
        let limit = self.grid_page_size;
        let q = match kind {
            DbKind::Mongodb => format!("{table_name}.find({{}})"),
            DbKind::Couchdb => "{\n  \"selector\": {},\n  \"limit\": 50\n}".into(),
            DbKind::Neo4j => format!("MATCH (n:`{table_name}`) RETURN n LIMIT {limit}"),
            DbKind::Mssql | DbKind::AzureSql => format!("SELECT TOP {limit} * FROM {qt};"),
            DbKind::Oracle | DbKind::Db2 => {
                format!("SELECT * FROM {qt} FETCH FIRST {limit} ROWS ONLY")
            }
            DbKind::Redis => "SCAN 0 COUNT 50".into(),
            _ => format!("SELECT * FROM {qt} LIMIT {limit};"),
        };
        *self.active_sql_mut() = q;
        self.grid_table_name = Some(table_name.to_string());
        self.grid_page = 0;
        self.db_subview = DbSubView::Grid;
        self.run_active_query();
    }

    pub fn run_sql_script(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        let raw = self.active_sql().to_string();
        let script = bapu_core::db_sql_tools::bind_sql_params(&raw, &self.sql_param_values);
        self.db_busy = true;
        let db_name = db.name.clone();
        let preview: String = script.lines().next().unwrap_or("Script").chars().take(80).collect();
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let result = bapu_core::db_sql_tools::execute_sql_script(&*driver, &db, &script, true)
                .await
                .map_err(|e| e.to_string());
            let ok = result.as_ref().is_ok_and(|r| r.failed == 0);
            let history_entry = HistoryItem {
                id: uuid::Uuid::new_v4().to_string(),
                kind: "sql".into(),
                title: format!("[SCRIPT] {preview}"),
                subtitle: db_name,
                status: Some(if ok { 200 } else { 500 }),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };
            let _ = tx.send(AsyncEvent::SqlScriptCompleted {
                result,
                history_entry,
            });
        });
    }

    pub fn execute_ddl_and_refresh_schema(&mut self, ddl: String) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let res = bapu_core::db_sql_tools::execute_sql_script(&*driver, &db, &ddl, true)
                .await
                .and_then(|rep| {
                    if rep.failed > 0 {
                        let msg = rep
                            .items
                            .iter()
                            .find_map(|i| i.error.clone())
                            .unwrap_or_else(|| "DDL failed".into());
                        anyhow::bail!("{msg}");
                    }
                    Ok(())
                })
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::DdlExecuted(res));
        });
    }

    pub fn reparse_import_preview(&mut self, target_cols: &[String]) {
        match bapu_core::db_export_import::parse_import_payload(&self.import_raw_text) {
            Ok((src_cols, _)) => {
                self.import_col_mapping.clear();
                for sc in &src_cols {
                    let matched = target_cols
                        .iter()
                        .find(|tc| tc.eq_ignore_ascii_case(sc))
                        .cloned()
                        .unwrap_or_else(|| sc.clone());
                    self.import_col_mapping.insert(sc.clone(), matched);
                }
                self.import_source_cols = src_cols;
                self.import_status = None;
            }
            Err(e) => {
                self.import_source_cols.clear();
                self.import_status = Some(format!("Parse error: {e}"));
            }
        }
    }

    pub fn run_data_import(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        let table = self.import_target_table.trim().to_string();
        if table.is_empty() {
            self.import_status = Some("Select a target table first.".into());
            return;
        }
        let rows = match bapu_core::db_export_import::parse_import_payload(&self.import_raw_text) {
            Ok((_, r)) => r,
            Err(e) => {
                self.import_status = Some(format!("Import parse error: {e}"));
                return;
            }
        };
        let mutations =
            bapu_core::db_export_import::build_import_mutations(&rows, &self.import_col_mapping);
        if mutations.is_empty() {
            self.import_status = Some("No mapped non-empty columns to import.".into());
            return;
        }
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let res = driver
                .mutate_batch(&db, &table, &mutations)
                .await
                .map(|b| b.results.len())
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::DataImportCompleted(res));
        });
    }

    pub fn run_schema_compare(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(src_db) = self.databases.get(self.compare_source_db_idx).cloned() else {
            return;
        };
        let Some(tgt_db) = self.databases.get(self.compare_target_db_idx).cloned() else {
            return;
        };
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let src_drv = bapu_core::db::driver_for(src_db.kind);
            let tgt_drv = bapu_core::db::driver_for(tgt_db.kind);
            let res = async {
                let s_schema = src_drv.get_schema(&src_db).await?;
                let t_schema = tgt_drv.get_schema(&tgt_db).await?;
                Ok::<_, anyhow::Error>(bapu_core::db_diff::compare_schemas(
                    &s_schema,
                    &t_schema,
                    tgt_db.kind,
                ))
            }
            .await
            .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::SchemaCompareCompleted(res));
        });
    }

    pub fn run_data_compare(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(src_db) = self.databases.get(self.compare_source_db_idx).cloned() else {
            return;
        };
        let Some(tgt_db) = self.databases.get(self.compare_target_db_idx).cloned() else {
            return;
        };
        let tbl = self.compare_table_name.trim().to_string();
        if tbl.is_empty() {
            return;
        }
        let pk_cols: Vec<String> = src_db
            .tables
            .iter()
            .find(|t| t.name == tbl)
            .map(|t| {
                t.columns
                    .iter()
                    .filter(|c| c.is_primary_key)
                    .map(|c| c.name.clone())
                    .collect()
            })
            .unwrap_or_default();

        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let src_drv = bapu_core::db::driver_for(src_db.kind);
            let tgt_drv = bapu_core::db::driver_for(tgt_db.kind);
            let res = async {
                let qs = if src_db.kind == DbKind::Mongodb {
                    format!("{tbl}.find({{}})")
                } else {
                    format!("SELECT * FROM {}", quote_dialect_ident(&tbl, src_db.kind))
                };
                let qt = if tgt_db.kind == DbKind::Mongodb {
                    format!("{tbl}.find({{}})")
                } else {
                    format!("SELECT * FROM {}", quote_dialect_ident(&tbl, tgt_db.kind))
                };
                let src_rows = src_drv.query(&src_db, &qs).await?;
                let tgt_rows = tgt_drv.query(&tgt_db, &qt).await?;
                Ok::<_, anyhow::Error>(bapu_core::db_diff::compare_table_data(
                    &src_rows,
                    &tgt_rows,
                    &pk_cols,
                    &tbl,
                    tgt_db.kind,
                ))
            }
            .await
            .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::DataCompareCompleted(res));
        });
    }

    pub fn run_mock_data_insert(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        let Some(tbl) = db.tables.iter().find(|t| t.name == self.mock_gen_table).cloned() else {
            return;
        };
        let count = self.mock_gen_count.max(1);
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            // Automatically fetch valid parent PK values for every FK constraint on `tbl`!
            let mut fk_pools: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            for fk in &tbl.foreign_keys {
                let q_ref_t = quote_dialect_ident(&fk.referenced_table, db.kind);
                let q_ref_c = quote_dialect_ident(&fk.referenced_column, db.kind);
                if let Ok(parent_qr) = driver
                    .query(&db, &format!("SELECT {q_ref_c} FROM {q_ref_t} LIMIT 200"))
                    .await
                {
                    let pool: Vec<serde_json::Value> = parent_qr
                        .rows
                        .into_iter()
                        .filter_map(|mut r| r.remove(&fk.referenced_column))
                        .filter(|v| !v.is_null())
                        .collect();
                    if !pool.is_empty() {
                        fk_pools.insert(fk.column.clone(), pool);
                    }
                }
            }
            let mutations =
                bapu_core::db_mock_gen::generate_mock_mutations(&tbl, count, &fk_pools, false);
            let res = driver
                .mutate_batch(&db, &tbl.name, &mutations)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::BatchMutationCompleted(res));
        });
    }

    pub fn refresh_server_health(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let res = bapu_core::db_admin::fetch_server_health(&*driver, &db)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::ServerHealthLoaded(res));
        });
    }

    pub fn run_active_query(&mut self) {
        let sql = self.active_sql().to_string();
        self.run_sql_text(sql);
    }

    pub fn explain_active_query(&mut self) {
        let Some(db) = self.databases.get(self.active_db_index) else {
            return;
        };
        let raw = self.active_sql().trim();
        if raw.is_empty() {
            return;
        }
        let explain_sql = match db.kind {
            DbKind::Postgres | DbKind::Yugabytedb | DbKind::Timescaledb => {
                format!("EXPLAIN (ANALYZE, COSTS, VERBOSE, BUFFERS, FORMAT JSON) {raw}")
            }
            DbKind::Cockroachdb | DbKind::Redshift | DbKind::Trino | DbKind::Clickhouse
            | DbKind::Snowflake | DbKind::Databricks => format!("EXPLAIN {raw}"),
            DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => format!("EXPLAIN FORMAT=JSON {raw}"),
            DbKind::Sqlite | DbKind::Libsql => format!("EXPLAIN QUERY PLAN {raw}"),
            DbKind::Oracle => format!("EXPLAIN PLAN FOR {raw}"),
            DbKind::Neo4j => format!("EXPLAIN {raw}"),
            _ => raw.to_string(),
        };
        self.run_sql_text(explain_sql);
    }

    pub fn test_active_connection(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            self.db_status = Some("No database connection selected.".into());
            return;
        };
        self.db_busy = true;
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let result = driver.test_connection(&db).await.map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::ConnectionTest(result));
        });
    }

    pub fn load_active_schema(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            return;
        };
        self.db_busy = true;
        let db_id = db.id.clone();
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let result = driver.get_schema(&db).await.map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::SchemaLoaded { db_id, result });
        });
    }

    pub fn run_ai_copilot(&mut self) {
        if self.ai_busy || self.ai_prompt.trim().is_empty() {
            return;
        }
        self.ai_busy = true;
        self.ai_error = None;

        let schema_hint = self
            .databases
            .get(self.active_db_index)
            .map(|db| {
                let tables: Vec<String> = db
                    .tables
                    .iter()
                    .map(|t| {
                        let cols: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
                        format!("{}({})", t.name, cols.join(", "))
                    })
                    .collect();
                format!("Active DB ({:?}): {}", db.kind, tables.join("; "))
            })
            .unwrap_or_default();

        if self.ai_provider != AiProvider::Ollama {
            self.storage
                .save_ai_api_key(self.ai_provider.slug(), &self.ai_api_key);
        }

        let req = AiRequest {
            provider: self.ai_provider,
            base_url: self.ai_base_url.clone(),
            model: self.ai_model.clone(),
            api_key: self.ai_api_key.clone(),
            system_prompt: format!(
                "You are Bapu Studio's concise developer assistant. Return clean SQL, JSON payloads, or test scripts without unnecessary prose. {schema_hint}"
            ),
            user_prompt: self.ai_prompt.clone(),
        };
        let client = self.http_client.clone();
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let res = ai_copilot::generate(&client, &req)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::AiResponse(res));
        });
    }

    pub fn connect_stream(&mut self) {
        self.disconnect_stream();
        let url = self.stream_url.trim().to_string();
        if url.is_empty() {
            return;
        }
        self.stream_error = None;
        let tx = self.stream_events_tx.clone();
        let handle = match self.stream_kind {
            StreamKind::WebSocket => stream_client::connect_websocket(&self.runtime, url, tx),
            StreamKind::Sse => {
                stream_client::connect_sse(&self.runtime, self.http_client.clone(), url, tx)
            }
        };
        self.stream_handle = Some(handle);
    }

    pub fn disconnect_stream(&mut self) {
        if let Some(mut h) = self.stream_handle.take() {
            h.disconnect();
        }
        self.stream_connected = false;
    }

    pub fn send_stream_message(&mut self) {
        let payload = self.stream_outbound.clone();
        if payload.trim().is_empty() {
            return;
        }
        if let Some(ref handle) = self.stream_handle {
            if let Err(e) = handle.send(payload) {
                self.stream_error = Some(e);
            }
        }
    }

    fn poll_async_results(&mut self) {
        let mut should_refresh_query = false;
        let mut should_reload_schema = false;
        let mut run_playground_select = false;

        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                AsyncEvent::ApiCompleted {
                    result,
                    updated_env_vars,
                    updated_globals,
                    updated_col_vars,
                    new_cookies,
                    history_entry,
                } => {
                    self.api_busy = false;
                    let vars_changed = !updated_env_vars.is_empty()
                        || !updated_globals.is_empty()
                        || !updated_col_vars.is_empty();
                    if let Some(env) = self.environments.get_mut(self.active_env_index) {
                        for (k, v) in updated_env_vars {
                            apply_kv(&mut env.variables, &k, &v);
                        }
                    }
                    for (k, v) in updated_globals {
                        apply_kv(&mut self.globals, &k, &v);
                    }
                    if let Some(cid) = self.active_request.collection_id.clone() {
                        if let Some(col) = self.collections.iter_mut().find(|c| c.id == cid) {
                            for (k, v) in updated_col_vars {
                                apply_kv(&mut col.variables, &k, &v);
                            }
                        }
                    }
                    self.cookie_jar.entries = new_cookies;
                    if vars_changed {
                        self.persist();
                    } else {
                        let _ = self.storage.save_cookies(&self.cookie_jar.entries);
                    }
                    self.push_history(history_entry);
                    match result {
                        Ok(resp) => {
                            self.last_response = Some(resp);
                            self.api_error = None;
                        }
                        Err(e) => {
                            self.last_response = None;
                            self.api_error = Some(format!("Request failed: {e}"));
                        }
                    }
                }
                AsyncEvent::OAuth2TokenFetched(res) => {
                    self.api_busy = false;
                    match res {
                        Ok(tok) => {
                            self.active_request.auth_config.token = Some(tok);
                            self.api_error = None;
                        }
                        Err(e) => {
                            self.api_error = Some(format!("OAuth 2.0 token error: {e}"));
                        }
                    }
                }
                AsyncEvent::RunnerCompleted { result, cookies } => {
                    self.runner_busy = false;
                    self.cookie_jar.entries = cookies;
                    let _ = self.storage.save_cookies(&self.cookie_jar.entries);
                    match result {
                        Ok(report) => {
                            if let Some(env) = self.environments.get_mut(self.active_env_index) {
                                *env = report.updated_env.clone();
                            }
                            self.globals = report.updated_globals.clone();
                            self.runner_report = Some(report);
                            self.runner_error = None;
                            self.persist();
                        }
                        Err(e) => {
                            self.runner_error = Some(e);
                        }
                    }
                }
                AsyncEvent::MockServerStarted(res) => match res {
                    Ok(handle) => {
                        self.mock_handle = Some(handle);
                        self.mock_error = None;
                    }
                    Err(e) => {
                        self.mock_error = Some(e);
                    }
                },
                AsyncEvent::QueryResult {
                    result,
                    history_entry,
                } => {
                    self.db_busy = false;
                    self.push_history(history_entry);
                    self.last_query_result = Some(result);
                }
                AsyncEvent::ConnectionTest(result) => {
                    self.db_busy = false;
                    if result.is_ok() {
                        if let Some(db) = self.databases.get_mut(self.active_db_index) {
                            db.is_connected = true;
                        }
                    }
                    self.db_status = Some(match result {
                        Ok(msg) => msg,
                        Err(e) => format!("Connection failed: {e}"),
                    });
                }
                AsyncEvent::SchemaLoaded { db_id, result } => {
                    self.db_busy = false;
                    match result {
                        Ok(tables) => {
                            if let Some(db) = self.databases.iter_mut().find(|d| d.id == db_id) {
                                let count = tables.len();
                                db.tables = tables;
                                db.is_connected = true;
                                self.db_status =
                                    Some(format!("Loaded {count} table(s)/collection(s)."));
                            }
                        }
                        Err(e) => {
                            self.db_status = Some(format!("Schema load failed: {e}"));
                        }
                    }
                }
                AsyncEvent::PlaygroundSeeded { db } => {
                    self.db_busy = false;
                    self.databases.push(db);
                    self.active_db_index = self.databases.len() - 1;
                    *self.active_sql_mut() = "SELECT * FROM \"users\" LIMIT 25;".into();
                    self.grid_table_name = Some("users".into());
                    self.db_status = Some(
                        "SQLite Playground ready — edit any cell to stage transactional changes."
                            .into(),
                    );
                    self.persist();
                    run_playground_select = true;
                }
                AsyncEvent::BatchMutationCompleted(result) => {
                    self.db_busy = false;
                    match result {
                        Ok(batch) => {
                            let n = batch.results.len();
                            let mode = if batch.atomic {
                                "atomically"
                            } else {
                                "sequentially (non-atomic)"
                            };
                            self.db_status = Some(format!(
                                "Saved {n} change(s) {mode} in {}ms.{}",
                                batch.execution_time_ms,
                                batch
                                    .message
                                    .as_deref()
                                    .map(|m| format!(" {m}"))
                                    .unwrap_or_default()
                            ));
                            self.db_mock_gen_open = false;
                            self.revert_grid_changes();
                            should_refresh_query = true;
                            should_reload_schema = true;
                        }
                        Err(e) => {
                            self.db_status = Some(format!("Transaction rolled back: {e}"));
                        }
                    }
                }
                AsyncEvent::SqlScriptCompleted {
                    result,
                    history_entry,
                } => {
                    self.db_busy = false;
                    self.push_history(history_entry);
                    match result {
                        Ok(rep) => {
                            if let Some(ref last) = rep.last_result {
                                self.last_query_result = Some(Ok(last.clone()));
                            }
                            self.db_status = Some(format!(
                                "Script finished: {}/{} statements succeeded ({}ms).",
                                rep.succeeded, rep.total_statements, rep.total_elapsed_ms
                            ));
                            self.last_script_report = Some(rep);
                            self.db_subview = DbSubView::ConsoleLog;
                            should_reload_schema = true;
                        }
                        Err(e) => {
                            self.db_status = Some(format!("Script failed: {e}"));
                        }
                    }
                }
                AsyncEvent::DdlExecuted(res) => {
                    self.db_busy = false;
                    match res {
                        Ok(()) => {
                            self.table_designer_open = false;
                            self.db_status =
                                Some("DDL executed successfully — refreshing schema...".into());
                            should_reload_schema = true;
                        }
                        Err(e) => {
                            self.db_status = Some(format!("DDL execution failed: {e}"));
                        }
                    }
                }
                AsyncEvent::DataImportCompleted(res) => {
                    self.db_busy = false;
                    match res {
                        Ok(n) => {
                            self.import_status =
                                Some(format!("Imported {n} row(s) atomically!"));
                            self.db_status =
                                Some(format!("Imported {n} row(s) successfully."));
                            self.db_import_open = false;
                            should_refresh_query = true;
                            should_reload_schema = true;
                        }
                        Err(e) => {
                            self.import_status = Some(format!("Import failed (rolled back): {e}"));
                        }
                    }
                }
                AsyncEvent::SchemaCompareCompleted(res) => {
                    self.db_busy = false;
                    match res {
                        Ok(rep) => self.schema_diff_report = Some(rep),
                        Err(e) => self.db_status = Some(format!("Schema compare failed: {e}")),
                    }
                }
                AsyncEvent::DataCompareCompleted(res) => {
                    self.db_busy = false;
                    match res {
                        Ok(rep) => self.data_diff_report = Some(rep),
                        Err(e) => self.db_status = Some(format!("Data compare failed: {e}")),
                    }
                }
                AsyncEvent::ServerHealthLoaded(res) => {
                    self.db_busy = false;
                    self.server_health = Some(res);
                }
                AsyncEvent::AiResponse(result) => {
                    self.ai_busy = false;
                    match result {
                        Ok(text) => {
                            self.ai_output = text;
                            self.ai_error = None;
                        }
                        Err(e) => {
                            self.ai_error = Some(e);
                        }
                    }
                }
            }
        }

        if should_refresh_query || run_playground_select {
            self.run_active_query();
        }
        if should_reload_schema && !self.db_busy {
            self.load_active_schema();
        }

        while let Ok(hit) = self.mock_hits_rx.try_recv() {
            self.mock_hits.insert(0, hit);
            self.mock_hits.truncate(200);
        }

        while let Ok(ev) = self.stream_events_rx.try_recv() {
            match ev {
                StreamEvent::Connected => {
                    self.stream_connected = true;
                    self.stream_error = None;
                }
                StreamEvent::Frame(f) => {
                    self.stream_frames.push(f);
                    if self.stream_frames.len() > 500 {
                        self.stream_frames.remove(0);
                    }
                }
                StreamEvent::Error(e) => {
                    self.stream_error = Some(e.clone());
                    self.stream_frames.push(StreamFrame {
                        direction: FrameDirection::System,
                        payload: format!("Error: {e}"),
                        timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
                    });
                }
                StreamEvent::Closed => {
                    self.stream_connected = false;
                    self.stream_handle = None;
                }
            }
        }
    }

    fn push_history(&mut self, item: HistoryItem) {
        self.history.insert(0, item);
        self.history.truncate(200);
        let _ = self.storage.save_history(&self.history);
    }

    pub fn persist(&self) {
        let _ = self.storage.save_collections(&self.collections);
        let _ = self.storage.save_databases(&self.databases);
        let _ = self.storage.save_environments(&self.environments);
        let _ = self.storage.save_globals(&self.globals);
        let _ = self.storage.save_cookies(&self.cookie_jar.entries);
        let _ = self.storage.save_sql_snippets(&self.sql_snippets);
        let _ = self.storage.save_sql_tabs(&self.sql_tabs);
        let _ = self.storage.save_ui_prefs(&bapu_core::storage::UiPrefs {
            dark_mode: self.dark_mode,
        });
        let _ = self.storage.save_history(&self.history);
    }
}

fn parse_cell_input(raw: &str) -> serde_json::Value {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("null") {
        return serde_json::Value::Null;
    }
    if trimmed == "true" {
        return serde_json::Value::Bool(true);
    }
    if trimmed == "false" {
        return serde_json::Value::Bool(false);
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(n);
        }
    }
    serde_json::Value::String(raw.to_string())
}

fn detect_single_table(sql: &str) -> Option<String> {
    let upper = sql.to_ascii_uppercase();
    if !upper.trim_start().starts_with("SELECT")
        || upper.contains(" JOIN ")
        || upper.contains(" GROUP BY ")
    {
        return None;
    }
    let from_idx = upper.find(" FROM ")?;
    let after_from = sql[from_idx + 6..].trim_start();
    let token = after_from
        .split(|c: char| c.is_whitespace() || c == ';')
        .next()?
        .trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']');
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn apply_kv(kvs: &mut Vec<KeyValuePair>, key: &str, value: &str) {
    if let Some(existing) = kvs.iter_mut().find(|v| v.key == key) {
        existing.value = value.to_string();
    } else {
        kvs.push(KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            value: value.to_string(),
            enabled: true,
            is_secret: Some(false),
        });
    }
}

impl eframe::App for BapuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::theme::apply_theme(ctx, self.dark_mode);

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.quick_find_open = !self.quick_find_open;
        }
        let trigger_run = ctx.input(|i| {
            (i.modifiers.command && i.key_pressed(egui::Key::Enter))
                || i.key_pressed(egui::Key::F5)
        });
        if trigger_run {
            match self.tab {
                Tab::Api => self.send_active_request(),
                Tab::Database => self.run_active_query(),
                _ => {}
            }
        }
        if self.tab == Tab::Database {
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
                self.commit_grid_changes();
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::X)) {
                self.run_sql_script();
            }
        }

        self.poll_async_results();
        if self.api_busy
            || self.db_busy
            || self.ai_busy
            || self.runner_busy
            || self.stream_connected
            || self.mock_handle.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        ui::top_bar(self, ctx);
        ui::side_bar(self, ctx);

        let pal = self.palette();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_app)
                    .inner_margin(egui::Margin::same(16.0_f32)),
            )
            .show(ctx, |ui| match self.tab {
                Tab::Api => ui::api_studio::render(self, ui),
                Tab::Database => ui::database_studio::render(self, ui),
                Tab::Streams => ui::stream_studio::render(self, ui),
                Tab::Secrets => ui::secrets_studio::render(self, ui),
                Tab::History => ui::history_studio::render(self, ui),
            });

        if self.ai_modal_open {
            ui::ai_copilot::render_window(self, ctx);
        }
        if self.quick_find_open {
            ui::render_quick_find(self, ctx);
        }
        ui::runner_modals::render_all(self, ctx);
        ui::db_modals::render_all(self, ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_mock();
        self.disconnect_stream();
        self.persist();
    }
}
