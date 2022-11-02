use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize)]
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

#[derive(Debug, PartialEq, Eq, Clone, Archive, Deserialize, Serialize, serde::Serialize)]
#[archive_attr(derive(CheckBytes, Debug))]
pub(crate) struct IndexData {
    /// Contains all necessary information to add a document to a query's results
    pub source_query: u64,
    pub key: u64,
    pub document_id: u64,
    pub name: Option<String>,
    pub match_indices: Vec<[usize; 2]>,
    pub score: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchData {
    snippet: String,
    score: u32,
}
