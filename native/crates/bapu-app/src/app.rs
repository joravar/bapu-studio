use crate::ui;
use bapu_core::types::{ApiRequest, ApiResponse, AuthConfig, AuthType, BodyType, Collection, DatabaseConnection, DbKind, Environment, HttpMethod, KeyValuePair, QueryResult};
use bapu_core::Storage;
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Handle;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Tab {
    Api,
    Database,
    Secrets,
    History,
}

/// Results of background async work land here and get drained once per frame in `update()`.
pub enum AsyncEvent {
    ApiResponse(Result<ApiResponse, String>),
    QueryResult(Result<QueryResult, String>),
    ConnectionTest(Result<String, String>),
}

pub struct BapuApp {
    pub storage: Storage,
    pub runtime: Handle,
    pub http_client: reqwest::Client,

    pub tab: Tab,

    pub collections: Vec<Collection>,
    pub active_request: ApiRequest,
    pub last_response: Option<ApiResponse>,
    pub api_busy: bool,

    pub databases: Vec<DatabaseConnection>,
    pub active_db_index: usize,
    pub sql_text: String,
    pub last_query_result: Option<Result<QueryResult, String>>,
    pub db_busy: bool,
    pub db_status: Option<String>,

    pub environments: Vec<Environment>,
    pub active_env_index: usize,

    pub events_tx: mpsc::Sender<AsyncEvent>,
    pub events_rx: mpsc::Receiver<AsyncEvent>,
}

fn blank_request() -> ApiRequest {
    ApiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        name: "New Request".into(),
        method: HttpMethod::GET,
        url: String::new(),
        params: vec![],
        headers: vec![KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: "Accept".into(),
            value: "application/json".into(),
            enabled: true,
            is_secret: None,
        }],
        body_type: BodyType::None,
        body_content: String::new(),
        graphql_query: None,
        graphql_variables: None,
        auth_type: AuthType::None,
        auth_config: AuthConfig::default(),
        pre_request_script: None,
        test_script: None,
        tests: None,
        collection_id: None,
    }
}

fn default_environment() -> Environment {
    Environment { id: "env-default".into(), name: "Default".into(), variables: vec![] }
}

impl BapuApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: Handle) -> Self {
        let storage = Storage::new().expect("failed to initialize the local data directory");

        let collections = storage.load_collections().unwrap_or_default();
        let databases = storage.load_databases().unwrap_or_default();
        let mut environments = storage.load_environments().unwrap_or_default();
        if environments.is_empty() {
            environments.push(default_environment());
        }

        let active_request = collections
            .first()
            .and_then(|c| c.requests.first().cloned())
            .unwrap_or_else(blank_request);

        let (events_tx, events_rx) = mpsc::channel();

        Self {
            storage,
            runtime,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build the HTTP client"),
            tab: Tab::Api,
            collections,
            active_request,
            last_response: None,
            api_busy: false,
            databases,
            active_db_index: 0,
            sql_text: "SELECT 1".into(),
            last_query_result: None,
            db_busy: false,
            db_status: None,
            environments,
            active_env_index: 0,
            events_tx,
            events_rx,
        }
    }

    pub fn send_active_request(&mut self) {
        if self.active_request.url.trim().is_empty() || self.api_busy {
            return;
        }
        self.api_busy = true;
        let client = self.http_client.clone();
        let request = self.active_request.clone();
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let result = bapu_core::http_client::send_request(&client, &request)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::ApiResponse(result));
        });
    }

    pub fn run_active_query(&mut self) {
        if self.db_busy {
            return;
        }
        let Some(db) = self.databases.get(self.active_db_index).cloned() else {
            self.last_query_result = Some(Err("No database connection selected.".into()));
            return;
        };
        self.db_busy = true;
        let sql = self.sql_text.clone();
        let tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let driver = bapu_core::db::driver_for(db.kind);
            let result = driver.query(&db, &sql).await.map_err(|e| e.to_string());
            let _ = tx.send(AsyncEvent::QueryResult(result));
        });
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

    fn poll_async_results(&mut self) {
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                AsyncEvent::ApiResponse(result) => {
                    self.api_busy = false;
                    match result {
                        Ok(resp) => self.last_response = Some(resp),
                        Err(e) => {
                            self.last_response = None;
                            self.db_status = Some(format!("Request failed: {e}"));
                        }
                    }
                }
                AsyncEvent::QueryResult(result) => {
                    self.db_busy = false;
                    self.last_query_result = Some(result);
                }
                AsyncEvent::ConnectionTest(result) => {
                    self.db_busy = false;
                    self.db_status = Some(match result {
                        Ok(msg) => msg,
                        Err(e) => format!("Connection failed: {e}"),
                    });
                }
            }
        }
    }

    pub fn persist(&self) {
        let _ = self.storage.save_collections(&self.collections);
        let _ = self.storage.save_databases(&self.databases);
        let _ = self.storage.save_environments(&self.environments);
    }
}

impl eframe::App for BapuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_async_results();
        if self.api_busy || self.db_busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        ui::top_bar(self, ctx);
        ui::side_bar(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Api => ui::api_studio::render(self, ui),
            Tab::Database => ui::database_studio::render(self, ui),
            Tab::Secrets => ui::secrets_studio::render(self, ui),
            Tab::History => ui::history_studio::render(self, ui),
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }
}

