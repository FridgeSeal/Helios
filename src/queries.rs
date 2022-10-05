use typed_id::TypedId;

use crate::search::TextSource;

pub(crate) type PersistentQueryId = TypedId<u64, PersistentQuery>;
pub(crate) type IndexId = TypedId<u32, IndexData>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct PersistentQuery {
    pub query: String,         // the query the user makes
    pub id: PersistentQueryId, // prefix/namespace to store stuff in database
    pub score_threshold: i64,
    result_count: u32,
}

impl PersistentQuery {
    pub fn new_with_key(q: impl Into<String>, key: u64) -> Self {
        Self {
            query: q.into(),
            id: key.into(),
            result_count: 0,
            score_threshold: 0,
        }
    }

    pub fn new(q: impl Into<String>) -> Self {
        Self {
            query: q.into(),
            id: rand::random::<u64>().into(),
            score_threshold: 200, // Need a good way of refining this
            result_count: 0,
        }
    }

    // pub fn search(&self, data: &TextSource) -> Option<Vec<IndexData>> {
    //     /// Given some `data`, if the data contains matches for the query,
    //     /// produce a `TextCandidate` structure that will cause the data to be added to the
    //     /// search results for that query
    //     unimplemented!() // TODO
    // }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IndexData {
    /// Contains all necessary information to add a document to a query's results
    pub source_query: PersistentQueryId,
    pub key: IndexId,
    pub document_id: u32,
    pub name: Option<String>,
    pub match_indices: Vec<[usize; 2]>, // not sure what we'd put here yet - snippet info?
    pub score: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchData {
    snippet: String,
    score: u32,
}
