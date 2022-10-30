use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};

use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};

use crate::{
    errors::ApiError,
    queries::{IndexData, PersistentQuery},
    search::TextSource,
};

pub async fn http_server(
    addr: SocketAddr,
    query_map: flashmap::WriteHandle<u64, PersistentQuery>,
    doc_channel: tachyonix::Sender<TextSource>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(State {
        query_map,
        document_channel: doc_channel,
    }));
    let app = Router::new()
        .route("/", get(healthcheck))
        .route("/healthcheck", get(healthcheck))
        .route("/query/get/:query_id", get(get_query))
        .route("/query/submit", post(submit_query))
        .route("/document/submit", post(submit_document))
        .route("/query/get_results/:query_id", get(get_query_results))
        .layer(Extension(state));
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
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<Json<QuerySubmitResponse>, ApiError> {
    // Todo: separate out validation logic from actual path handler
    if !(payload.query_string.is_empty() || payload.threshold <= 0) {
        return Err(ApiError::QuerySubmission);
    }
    // Additionally, the lock ownership section should be segmented into an outer and an inner function
    // in which the inner does not care about Mutex/Lock semantics
    let query: PersistentQuery = payload.into();
    let mut state_lock = state.lock().await;
    // We could make this lock-free if we implemented the poll-loop directly
    state_lock.query_map.guard().insert(query.id, query);
    Ok(Json(QuerySubmitResponse::succeeded()))
}

async fn submit_document(
    Json(text_payload): Json<TextSource>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<Json<DocumentSubmissionResult>, ApiError> {
    if text_payload.data.is_empty() {
        return Err(ApiError::DocSubmission);
    }
    let mut state_lock = state.lock().await;
    state_lock.document_channel.send(text_payload).await?;
    Ok(Json(DocumentSubmissionResult { successful: true }))
}

async fn get_query(
    Path(query_id): Path<u64>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<Json<PersistentQuery>, ApiError> {
    let mut state_guard = state.lock().await;
    // A bit less than ideal, we own the write-half of the map, we can't get a non-mutable, non-blocking
    // view into it. It'll do for now.
    let query_guard = state_guard.query_map.guard();
    query_guard
        .get(&query_id)
        .map(|x| Json(x.to_owned()))
        .ok_or(ApiError::NonExistentId)
}

async fn get_query_results(Path(query_id): Path<u64>) -> Result<Json<Vec<IndexData>>, ApiError> {
    Ok(Json(Vec::new()))
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
}

#[derive(Debug, Serialize)]
struct DocumentSubmissionResult {
    successful: bool,
}
