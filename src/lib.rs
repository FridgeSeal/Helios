// use crate::{errors::ApiError, search::TextSource};

use bytecheck::CheckBytes;
use rkyv::{validation::CheckTypeError, Archive, Deserialize, Serialize};

use thiserror::Error;
use tracing_subscriber::prelude::*;

#[tarpc::service]
pub trait Splinter {
    async fn hello(name: String) -> String;
    async fn healthcheck() -> String;
    async fn peer_health_capacity() -> LoadCapacityData;
    async fn get_query(query_id: u64) -> Result<PersistentQuery, TarkineError>;
    async fn submit_query(query: PersistentQuery);
    async fn submit_document(document: TextSource);
    async fn get_results(query_id: u64) -> Vec<IndexData>;
}

pub fn init_tracing(service_name: &str) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| format!("{service_name}=debug,tower_http=debug")),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoadCapacityData {
    pub conn_count: u32,
    pub query_count: u32,
}

#[derive(
    Archive,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    Serialize,
    Deserialize,
)]
// This will generate a PartialEq impl between our unarchived and archived types
#[archive(compare(PartialEq))]
// To use the safe API, you have to derive CheckBytes for the archived type
#[archive_attr(derive(CheckBytes, Debug))]
pub struct PersistentQuery {
    pub name: String,
    pub query: String, // the query the user makes - later this should be a dedicated structure for more complex query types
    pub id: u64,       // prefix/namespace to store stuff in database
    pub score_threshold: i64,
    result_count: u32,
}

impl PersistentQuery {
    pub fn new(id: u64, name: impl Into<String>, q: impl Into<String>, threshold: i64) -> Self {
        Self {
            name: name.into(),
            query: q.into(),
            id,
            score_threshold: threshold, // Need a good way of refining this
            result_count: 0,
        }
    }
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Archive,
    Deserialize,
    Serialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[archive_attr(derive(CheckBytes, Debug))]
pub struct IndexData {
    /// Contains all necessary information to add a document to a query's results
    pub source_query: u64,
    pub key: u64,
    pub document_id: u64,
    pub name: String,
    pub match_indices: Vec<[usize; 2]>,
    pub score: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchData {
    snippet: String,
    score: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TextSource {
    pub id: u64,
    pub name: String,
    pub data: String,
    // Feature idea -
}

impl TextSource {
    pub fn new(text: impl ToString, text_name: String) -> Self {
        Self {
            id: rand::random(),
            data: text.to_string(),
            name: text_name,
        }
    }

    // Lazy loading from supported sources, etc
}

#[tarpc::derive_serde]
#[derive(Debug, Error)]
pub enum TarkineError {
    #[error("Error reading/writing from storage layer")]
    Storage,
    #[error("Error communicating over network")]
    Network,
    #[error("Id not found or not correct format")]
    Id,
    #[error("Could not parse bytes")]
    Parsing,
}

impl From<sled::Error> for TarkineError {
    fn from(_: sled::Error) -> Self {
        Self::Storage
    }
}
