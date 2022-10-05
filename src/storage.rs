use std::{collections::HashMap, error};

use crate::queries::{IndexData, PersistentQuery, PersistentQueryId};

pub(crate) struct Storage {}

impl Storage {
    pub fn setup_storage() -> (MetadataStorage, IndexStorage) {
        (MetadataStorage::new(), IndexStorage::new())
    }
}

#[derive(Debug)]
pub(crate) struct MetadataStorage {
    metadata: HashMap<PersistentQueryId, PersistentQuery>,
}

#[derive(Debug)]
pub(crate) struct IndexStorage {
    indexes: HashMap<PersistentQueryId, Vec<IndexData>>,
}

impl MetadataStorage {
    pub fn new() -> Self {
        // let db = sled::open(path)?;
        // let metadata = db.open_tree("metadata")?;
        // let indexes = db.open_tree("data")?;
        let metadata = HashMap::with_capacity(1000);
        // let indexes = HashMap::new();
        Self { metadata }
    }

    pub fn store_query(&mut self, query: PersistentQuery) -> bool {
        // let query_bytes = rkyv::to_bytes::<_, 1024>(&query)?;
        // let _ = self
        //     .metadata
        //     .insert(query.query_id.as_bytes(), query_bytes.as_slice())?;
        self.metadata.insert(query.id, query);
        true
    }

    pub fn get_query(&self, key: PersistentQueryId) -> Option<&PersistentQuery> {
        // if let Ok(Some(query_buf)) = self.metadata.get(key) {
        //     rkyv::from_bytes::<PersistentQuery>(&query_buf).ok()
        // } else {
        //     None
        // }
        self.metadata.get(&key)
    }

    pub fn list_queries(&self) -> impl Iterator<Item = &PersistentQuery> {
        self.metadata.values().map(|x| x)
    }

    pub fn len(&self) -> usize {
        self.metadata.len()
    }
}

impl IndexStorage {
    pub fn new() -> Self {
        // let db = sled::open(path)?;
        // let metadata = db.open_tree("metadata")?;
        // let indexes = db.open_tree("data")?;
        // let metadata = HashMap::with_capacity(1000);
        let indexes = HashMap::new();
        Self { indexes }
    }

    pub fn store_index_data(&mut self, data: IndexData) {
        // let key = data.source_query.id + self.db.generate_id()?;
        // let bytes = rkyv::to_bytes::<_, 1024>(data)?;
        // let _insert_result = self.indexes.insert(key, bytes.as_slice())?;
        let index = self.indexes.entry(data.source_query);
        index.and_modify(|f| f.push(data)).or_default();
    }

    pub fn get_query_results(&self, query_id: PersistentQueryId) -> Option<&Vec<IndexData>> {
        self.indexes.get(&query_id)
    }
}
