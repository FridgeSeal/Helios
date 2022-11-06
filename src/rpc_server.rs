use anyhow::Ok;
use futures::{self, StreamExt};
use std::{net::{SocketAddr, IpAddr, Ipv6Addr}, time::Duration};
use tarpc::{context, server::{self, incoming::Incoming, Channel},
    tokio_serde::formats::Bincode};
use service::{init_tracing, World};
use tokio::time;
use rand::{
    distributions::{Distribution, Uniform},
    thread_rng,
};

// mod lib;

#[derive(Debug, Clone)]
struct WorldServer(SocketAddr);

#[tarpc::server]
impl World for WorldServer {
    async fn hello(self, _: context::Context, name: String) -> String {
    let sleep_time =
    Duration::from_millis(Uniform::new_inclusive(1, 10).sample(&mut thread_rng()));
    time::sleep(sleep_time).await;
    format!("Hello there {name}! You have connected to peer {}", self.0)
}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing("Splinter server")?;
    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8247);
    let mut listener = tarpc::serde_transport::tcp::listen(&server_addr, Bincode::default).await?;
    tracing::info!("Listening on port {}", listener.local_addr().port());
    listener.config_mut().max_frame_length(usize::MAX);
    listener
        // Ignore accept errors.
        .filter_map(|r| futures::future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        // Limit channels to 1 per IP.
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        // serve is generated by the service attribute. It takes as input any type implementing
        // the generated World trait.
        .map(|channel| {
            let server = WorldServer(channel.transport().peer_addr().unwrap());
            channel.execute(server.serve())
        })
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;
    Ok(())
}

//use axum::{
//    extract::{Extension, Path},
//    routing::{get, post},
//    Json, Router,
//};
//
//use futures::lock::Mutex;
//use serde::{Deserialize, Serialize};
//use tracing::{Span, event, Level};
//use tower_http::trace::TraceLayer;
//use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
//use serde::{Serialize, Deserialize};
//use std::{net::SocketAddr, sync::Arc};
//
//use crate::{
//    errors::ApiError,
//    queries::{IndexData, PersistentQuery},
//    search::TextSource,
//};
//
//
//pub async fn http_server(
//        addr: SocketAddr,
//        query_map: flashmap::WriteHandle<u64, PersistentQuery>,
//        doc_channel: tachyonix::Sender<TextSource>,
//        ) -> Result<(), Box<dyn std::error::Error>> {
//    let state = Arc::new(Mutex::new(State {
//        query_map,
//        document_channel: doc_channel,
//    }));
//    let app = Router::new()
//    .route("/", get(healthcheck))
//    .route("/healthcheck", get(healthcheck))
//    .route("/query/get/:query_id", get(get_query))
//    .route("/query/submit", post(submit_query))
//    .route("/document/submit", post(submit_document))
//    .route("/query/get_results/:query_id", get(get_query_results))
//    .layer(TraceLayer::new_for_http())
//    .layer(Extension(state));
//    event!(Level::INFO, message="Starting to listen", ?addr);
//    axum::Server::bind(&addr)
//    .http1_only(false)
//    .http2_only(true)
//    .tcp_nodelay(true)
//    .serve(app.into_make_service())
//    .await?;
//    Ok(())
//}
//
//async fn submit_query(
//        Json(payload): Json<SubmitQueryRequest>,
//        Extension(state): Extension<Arc<Mutex<State>>>,
//        ) -> Result<Json<QuerySubmitResponse>, ApiError> {
//    // Todo: separate out validation logic from actual path handler
//    if payload.query_string.is_empty() || payload.threshold <= 0 {
//        dbg!(payload);
//        return Err(ApiError::QuerySubmission);
//    }
//    // Additionally, the lock ownership section should be segmented into an outer and an inner function
//    // in which the inner does not care about Mutex/Lock semantics
//    let query: PersistentQuery = payload.into();
//    let mut state_lock = state.lock().await;
//    // We could make this lock-free if we implemented the poll-loop directly
//    state_lock.query_map.guard().insert(query.id, query);
//    Ok(Json(QuerySubmitResponse::succeeded()))
//}
//
//async fn submit_document(
//        Json(text_payload): Json<TextSource>,
//        Extension(state): Extension<Arc<Mutex<State>>>,
//        ) -> Result<Json<DocumentSubmissionResult>, ApiError> {
//    if text_payload.data.is_empty() {
//        return Err(ApiError::DocSubmission);
//    }
//    let mut state_lock = state.lock().await;
//    state_lock.document_channel.send(text_payload).await?;
//    Ok(Json(DocumentSubmissionResult { successful: true }))
//}
//
//async fn get_query(
//        Path(query_id): Path<u64>,
//        Extension(state): Extension<Arc<Mutex<State>>>,
//        ) -> Result<Json<PersistentQuery>, ApiError> {
//    let mut state_guard = state.lock().await;
//    // A bit less than ideal, we own the write-half of the map, we can't get a non-mutable, non-blocking
//    // view into it. It'll do for now.
//    let query_guard = state_guard.query_map.guard();
//    query_guard
//    .get(&query_id)
//    .map(|x| Json(x.to_owned()))
//    .ok_or(ApiError::NonExistentId)
//}
//
//async fn get_query_results(Path(query_id): Path<u64>) -> Result<Json<Vec<IndexData>>, ApiError> {
//    Ok(Json(Vec::new()))
//}
//
//async fn healthcheck() -> &'static str {
//    "Healthy!"
//}
//
//struct State {
//    query_map: flashmap::WriteHandle<u64, PersistentQuery>,
//    document_channel: tachyonix::Sender<TextSource>,
//}
//
//pub fn server_runtime(
//        addr: SocketAddr,
//        query_map: flashmap::WriteHandle<u64, PersistentQuery>,
//        doc_channel: tachyonix::Sender<TextSource>,
//        ) {
//    let runtime = tokio::runtime::Builder::new_current_thread()
//    .worker_threads(1)
//    .enable_io()
//    .thread_name("http server")
//    .build()
//    .expect("Couldn't build server");
//    runtime
//    .block_on(http_server(addr, query_map, doc_channel))
//    .expect("Server failed");
//}
//
//#[derive(Debug, Deserialize)]
//struct SubmitQueryRequest {
//    id: u64,
//    name: String,
//    query_string: String,
//    threshold: i64,
//}
//
//impl From<SubmitQueryRequest> for crate::PersistentQuery {
//    fn from(src: SubmitQueryRequest) -> Self {
//        PersistentQuery::new(src.id, src.name, src.query_string, src.threshold)
//    }
//}
//
//#[derive(Debug, Serialize)]
//struct QuerySubmitResponse {
//    successful: bool,
//}
//
//impl QuerySubmitResponse {
//    fn succeeded() -> Self {
//        Self { successful: true }
//    }
//}
//
//#[derive(Debug, Serialize)]
//struct DocumentSubmissionResult {
//    successful: bool,
//}
