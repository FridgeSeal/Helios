use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

use crate::{queries::PersistentQuery, search::TextSource};

pub async fn http_server(
    addr: SocketAddr,
    query_map: flashmap::WriteHandle<u64, PersistentQuery>,
    doc_channel: tachyonix::Sender<TextSource>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(RwLock::new(State {
        query_map,
        document_channel: doc_channel,
    }));
    let app = Router::new()
        .route("/", get(healthcheck))
        .route("/healthcheck", get(healthcheck))
        .route(
            "/query/get/:query_id",
            get({
                let shared_state = Arc::clone(&state);
                move |query_id| get_query(query_id, Arc::clone(&shared_state))
            }),
        )
        .route(
            "/query/submit",
            post({
                let shared_state = Arc::clone(&state);
                move |body| submit_query(body, Arc::clone(&shared_state))
            }),
        )
        .route(
            "/query/get_results/:query_id",
            get({
                let shared_state = Arc::clone(&state);
                move |body| get_query_results(body, Arc::clone(&shared_state))
            }),
        )
        .route(
            "/document/submit",
            post({
                let shared_state = Arc::clone(&state);
                move |body| submit_document(body, Arc::clone(&shared_state))
            }),
        );
    println!("Starting server!");
    axum::Server::bind(&addr)
        .http1_only(false)
        .http2_only(true)
        .tcp_nodelay(true)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn submit_query(
    Json(payload): Json<SubmitQueryRequest>,
    state: Arc<Mutex<State>>,
) -> Json<QuerySubmitResponse> {
    // Todo: separate out validation logic from actual path handler
    if !(!payload.query_string.is_empty() && payload.threshold > 0) {
        return Json(QuerySubmitResponse::failed());
    }
    // Additionally, the lock ownership section should be segmented into an outer and an inner function
    // in which the inner does not care about Mutex/Lock semantics
    let query: PersistentQuery = payload.into();
    let mut state_lock = state.lock().await;
    // We could make this lock-free if we implemented the poll-loop directly
    state_lock.query_map.guard().insert(query.id, query);
    Json(QuerySubmitResponse::succeeded())
}

async fn submit_document(
    Json(text_payload): Json<TextSource>,
    state: Arc<RwLock<State>>,
) -> Json<DocumentSubmissionResult> {
    if text_payload.data.is_empty() {
        return Json(DocumentSubmissionResult { successful: false });
    }
    let mut state_lock = state.write().await;
    match state_lock.document_channel.send(text_payload).await {
        Ok(_) => Json(DocumentSubmissionResult { successful: true }),
        Err(_) => Json(DocumentSubmissionResult { successful: false }),
    }
}

async fn get_query(query_id: u64, state: Arc<RwLock<State>>) -> Json<PersistentQuery> {
    let query = match state.read().await.query_map.guard().get(query_id) {
        Some(it) => it,
        _ => return Json,
    };
    Json(query.clone())
}
async fn healthcheck() -> &'static str {
    "Healthy!"
}

struct State {
    query_map: flashmap::WriteHandle<u64, PersistentQuery>,
    document_channel: tachyonix::Sender<TextSource>,
}

pub fn server_runtime(
    addr: SocketAddr,
    query_map: flashmap::WriteHandle<u64, PersistentQuery>,
    doc_channel: tachyonix::Sender<TextSource>,
) {
    println!("Starting server runtime");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_io()
        .thread_name("http server")
        .build()
        .expect("Couldn't build server");
    runtime
        .block_on(http_server(addr, query_map, doc_channel))
        .expect("Server failed");
}

#[derive(Debug, Deserialize)]
struct SubmitQueryRequest {
    query_string: String,
    threshold: i64,
}

impl From<SubmitQueryRequest> for crate::PersistentQuery {
    fn from(src: SubmitQueryRequest) -> Self {
        PersistentQuery::new(src.query_string, src.threshold)
    }
}

#[derive(Debug, Serialize)]
struct QuerySubmitResponse {
    successful: bool,
}

impl QuerySubmitResponse {
    fn succeeded() -> Self {
        Self { successful: true }
    }
    fn failed() -> Self {
        Self { successful: false }
    }
}

#[derive(Debug, Serialize)]
struct DocumentSubmissionResult {
    successful: bool,
}
