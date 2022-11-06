use futures_lite::AsyncWriteExt;
use glommio::{LocalExecutorBuilder, Placement, io::DmaStreamWriterBuilder, defer};
use itertools::Itertools;
use lib::{IndexData, PersistentQuery, TextSource};
use search::Searcher;
use tracing::{Level, event};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::{error, net::SocketAddr, path::{PathBuf}};

mod data_source;
mod errors;
mod search;
mod rpc_server;

use crate::rpc_server::server_runtime;

const DATA_PATH: &str = "output_data";

fn main() -> Result<(), Box<dyn error::Error>> {
    let db_path = PathBuf::from("splinter.data");
    tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "tarkine=debug,tower_http=debug".into()),
    ))
    .with(tracing_subscriber::fmt::layer())
    .init();
    let core_count = std::thread::available_parallelism()?;
    event!(Level::INFO, core_count);
    let bind_addr = SocketAddr::from(([127, 0, 0, 1], 8766));

    let (_write_map, read_map) = flashmap::with_capacity(1000);
    let (send_chan, recv_chan) = tachyonix::channel(1024);
    event!(Level::INFO, message="Starting API server thread");
    let server_threads =
        std::thread::spawn(move || server_runtime(bind_addr, db_path, send_chan));
        let shard1 = QueryShard {
            inner: read_map,
            engine: Searcher::new(),
        };
    event!(Level::INFO, message="Starting Indexing Thread");
    let processor_thread = LocalExecutorBuilder::new(Placement::Fixed(1))
        .spawn(|| index_runtime(shard1, recv_chan))?;

    match server_threads.join() {
        Ok(_) => event!(Level::INFO, message="Server thread has exited safely - shutting down"),
        Err(_) => event!(Level::ERROR, message="Server thread has crashed - restart application"),
    };
    processor_thread.join()?;
    /*
    TODO: Proper handling and error messages for thread exits.
    If we lose the server thread - do we drain the search threads and let the application exit?
    or do we attempt to restart the server thread?
    implement actual errors on the server and search threads too - if we lose a search thread, no point
    attempting to resurrect it if there's larger issues (permissions, resource, etc) preventing successful operation
    */

    Ok(())
    // Executor in current (in this case main) thread
    // let ex1 = LocalExecutorBuilder::new(Placement::Fixed(3)).make()?;

    // Executor dropped into spawned thread
    // let builder = LocalExecutorBuilder::new(Placement::Fixed(1));
    // let thread_handle = builder.name("Hello").spawn(|| async move {
    //     hello().await;
    // })?;
}

struct QueryShard {
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

fn form_path(query_id: u64, doc_id: u64) -> PathBuf {
    let mut path = PathBuf::from(DATA_PATH);
    let subpath = PathBuf::from(format!("{query_id}"));
    path.push(subpath);
    path.push(format!("{doc_id}.rkyv"));
    path
}

async fn index_runtime(shard: QueryShard, mut text_recv: tachyonix::Receiver<TextSource>) {
    loop {
        if let Ok(doc) = text_recv.recv().await {
            event!(Level::INFO, message="document received", thread_id=glommio::executor().id(), ?doc.id, ?doc.name);
            let doc_id=doc.id;
            let mut search_results = shard.search(doc).await;
            for index_data in search_results.drain(0..) {
                glommio::spawn_local(async move {
                    let output_path = form_path( index_data.source_query, index_data.document_id);
                    let debug_path = output_path.clone(); // fight me
                    defer!(event!(Level::INFO, message="Wrote results into file!", thread_id=glommio::executor().id(), ?doc_id, file_path=?debug_path));
                    std::fs::create_dir_all(output_path.parent().unwrap()).expect("Couldn't create dir manually"); // yay std lib functions
                    let dma_file = glommio::io::DmaFile::create(output_path).await.expect("Couldn't create file");
                    let mut writer = DmaStreamWriterBuilder::new(dma_file).build();
                    let index_buffer = rkyv::to_bytes::<_, 1024>(&index_data).unwrap(); // Live dangerously
                    writer.write_all(index_buffer.as_slice()).await.expect("Couldn't write file!");
                
                })
                .await;
            }
        }
    }
}
