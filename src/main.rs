use futures_lite::AsyncWriteExt;
use glommio::{LocalExecutorBuilder, Placement};
use itertools::Itertools;
use queries::{IndexData, PersistentQuery};
use search::{Searcher, TextSource};
use std::{error, net::SocketAddr, path::Path};
use tachyonix::{self};

mod data_source;
mod errors;
mod queries;
mod search;
mod server;
mod storage;

use crate::server::server_runtime;

fn main() -> Result<(), Box<dyn error::Error>> {
    let core_count = std::thread::available_parallelism()?;
    dbg!(&core_count);
    let bind_addr = SocketAddr::from(([127, 0, 0, 1], 8765));

    let (write_map, read_map) = flashmap::with_capacity(1000);
    let (send_chan, recv_chan) = tachyonix::channel(1024);

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
        Ok(_) => "Server thread has exited safely - shutting down",
        Err(_) => "Server thread has crashed - restart application",
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
                    sink.write(index_buffer.as_slice())
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
