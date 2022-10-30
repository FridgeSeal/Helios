use crate::queries::IndexData;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct IndexStorage {
    indexes: HashMap<u64, Vec<IndexData>>,
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

    pub async fn store_index_data(&mut self, data: IndexData) {
        // let key = data.source_query.id + self.db.generate_id()?;
        // let bytes = rkyv::to_bytes::<_, 1024>(data)?;
        // let _insert_result = self.indexes.insert(key, bytes.as_slice())?;
        let index = self.indexes.entry(data.source_query);
        index.and_modify(|f| f.push(data)).or_default();
    }

    pub fn get_query_results(&self, query_id: u64) -> Option<&Vec<IndexData>> {
        self.indexes.get(&query_id)
    }
}
