use futures_lite::{io::AsyncReadExt, AsyncWriteExt};
use glommio::{
    io::{DmaFile, DmaStreamReaderBuilder},
    LocalExecutorBuilder, Placement,
};
use itertools::Itertools;
use queries::{IndexData, PersistentQuery};
use search::{Searcher, TextSource};
use std::{error, net::SocketAddr, path::Path};
use tachyonix::{self};

mod data_source;
mod queries;
mod search;
mod server;
mod storage;

use crate::server::server_runtime;

fn main() -> Result<(), Box<dyn error::Error>> {
    let core_count = std::thread::available_parallelism()?;
    dbg!(&core_count);
    let bind_addr = SocketAddr::from(([127, 0, 0, 1], 8765));

    let (mut write_map, read_map) = flashmap::with_capacity(1000);
    let (mut send_chan, mut recv_chan) = tachyonix::channel(1024);

    let server_threads =
        std::thread::spawn(move || server_runtime(bind_addr, write_map, send_chan));
    let shard1 = QueryShard {
        id: rand::random(),
        inner: read_map,
        engine: Searcher::new(),
    };
    let processor_thread = LocalExecutorBuilder::new(Placement::Fixed(1))
        .spawn(|| index_runtime(shard1, recv_chan))?;

    match server_threads.join() {
        Ok(_) => "Suceeded in doing stuff with sever threads?",
        Err(_) => "Did not succeed",
    };
    processor_thread.join()?;

    Ok(())
    // Executor in current (in this case main) thread
    // let ex1 = LocalExecutorBuilder::new(Placement::Fixed(3)).make()?;

    // Executor dropped into spawned thread
    // let builder = LocalExecutorBuilder::new(Placement::Fixed(1));
    // let thread_handle = builder.name("Hello").spawn(|| async move {
    //     hello().await;
    // })?;
}

async fn document_producer_runtime(mut doc_stream: tachyonix::Sender<TextSource>) {
    println!(
        "starting document producer on executor id {}",
        glommio::executor().id()
    );
    let scan_dir = glommio::io::Directory::open("/home/fridgeseal/Projects/shards/data")
        .await
        .expect("Couldn't open directory");
    let dir_contents = scan_dir
        .sync_read_dir()
        .expect("Couldn't read into directory")
        .filter_map(Result::ok);
    let mut str_buffer = String::with_capacity(10000);
    for filename in dir_contents {
        println!(
            "submitting file from executor id: {}",
            glommio::executor().id()
        );
        let file = DmaFile::open(&filename.path())
            .await
            .expect("Couldn't open file: {file}");
        let mut reader = DmaStreamReaderBuilder::new(file).build();
        let _ = reader
            .read_to_string(&mut str_buffer)
            .await
            .expect("Couldn't read file");
        let fname = filename
            .file_name()
            .to_str()
            .unwrap_or_default()
            .to_string();
        let doc = TextSource::new(str_buffer.clone(), Some(fname));
        doc_stream.send(doc);
        str_buffer.clear();
        glommio::executor().yield_if_needed().await;
    }
}

struct QueryShard {
    id: u64,
    inner: flashmap::ReadHandle<u64, PersistentQuery>,
    engine: Searcher,
}

impl QueryShard {
    async fn search(&self, text: TextSource) -> Vec<IndexData> {
        // Later, a stream of results?
        self.inner
            .guard()
            .values()
            .filter_map(|q| self.engine.search(q, &text))
            .collect_vec()
    }
}

async fn index_runtime(shard: QueryShard, mut text_recv: tachyonix::Receiver<TextSource>) {
    loop {
        if let Ok(doc) = text_recv.recv().await {
            let mut search_results = shard.search(doc).await;
            for index_data in search_results.drain(0..) {
                glommio::spawn_local(async move {
                    let path = format!(
                        "/home/fridgeseal/Projects/Tarkine/output_data/{}_{}_{}.rkyv",
                        shard.id, index_data.source_query, index_data.document_id
                    );
                    let output_path = Path::new(&path);
                    let mut sink = glommio::io::ImmutableFileBuilder::new(output_path)
                        .build_sink()
                        .await
                        .unwrap();
                    let index_buffer = rkyv::to_bytes::<_, 1024>(&index_data).unwrap();
                    sink.write(&index_buffer.as_slice())
                        .await
                        .expect("Couldn't write buffer");
                    sink.seal().await.expect("Couldn't close seal");
                    println!("Wrote to file!");
                })
                .await;
            }
        }
    }
}
