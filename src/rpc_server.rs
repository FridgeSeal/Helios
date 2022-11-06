use futures::{self, StreamExt};
use lib::{IndexData, PersistentQuery, Splinter, TarkineError, TextSource};
use rand::{
    distributions::{Distribution, Uniform},
    thread_rng,
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tarpc::{
    context,
    server::{self, incoming::Incoming, Channel},
    tokio_serde::formats::Bincode,
};
use tokio::time;
use tracing::instrument;

#[derive(Debug, Clone)]
struct Server {
    addr: SocketAddr,
    doc_channel: tachyonix::Sender<TextSource>,
    query_map: sled::Tree,
}
impl Server {
    fn new(
        addr: SocketAddr,
        doc_channel: tachyonix::Sender<TextSource>,
        path: PathBuf,
    ) -> Result<Self, sled::Error> {
        tracing::info!(message="Starting RPC server state", database_path=?path);
        let db_cfg = sled::Config::default().use_compression(true).path(path);
        let db = db_cfg.open()?;
        let tree = db.open_tree("queries")?;
        Ok(Self {
            addr,
            doc_channel,
            query_map: tree,
        })
    }
}

#[tarpc::server]
impl Splinter for Server {
    #[instrument(skip(self))]
    async fn hello(self, _: context::Context, name: String) -> String {
        tracing::info!(message = "Responding to hello call", method = "hello");
        let sleep_time =
            Duration::from_millis(Uniform::new_inclusive(1, 10).sample(&mut thread_rng()));
        time::sleep(sleep_time).await;
        format!(
            "Hello there {name}! You have connected to peer {}",
            self.addr
        )
    }

    #[instrument]
    async fn healthcheck(self, _: context::Context) -> String {
        "(づ｡◕‿‿◕｡)づ  H E A L T H Y !".to_string()
    }

    #[instrument]
    async fn peer_health_capacity(self, _: context::Context) -> lib::LoadCapacityData {
        lib::LoadCapacityData {
            conn_count: 1,
            query_count: 0,
        }
    }

    #[instrument]
    async fn get_query(
        self,
        _: context::Context,
        query_id: u64,
    ) -> Result<PersistentQuery, TarkineError> {
        use rkyv::Deserialize;
        let Some(mut raw_query) = self.query_map.get(query_id.to_ne_bytes())? else {return Err(TarkineError::Id)};
        let archived =
            rkyv::check_archived_root::<PersistentQuery>(raw_query.as_mut()).map_err(|e| {
                tracing::error!(message = "Failed to deserialise rkyv bytes", underlying_error=?e);
                TarkineError::Parsing
            })?;
        archived
            .deserialize(&mut rkyv::Infallible)
            .map_err(|_e| TarkineError::Parsing)
    }

    async fn submit_query(self, _: context::Context, query: PersistentQuery) {
        unimplemented!() // TODO
    }

    async fn submit_document(self, _: context::Context, document: TextSource) {
        unimplemented!() // TODO
    }

    async fn get_results(self, _: context::Context, query_id: u64) -> Vec<IndexData> {
        unimplemented!() // TODO
    }
}

#[instrument]
async fn rpc_server(
    addr: SocketAddr,
    db_path: PathBuf,
    doc_channel: tachyonix::Sender<TextSource>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = tarpc::serde_transport::tcp::listen(&addr, Bincode::default).await?;
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
            let server = Server::new(
                channel.transport().peer_addr().unwrap(),
                doc_channel.clone(),
                db_path.clone(),
            )
            .expect("Couldn't start server state or open databases");
            channel.execute(server.serve())
        })
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;
    Ok(())
}

#[instrument]
pub fn server_runtime(
    addr: SocketAddr,
    db_path: PathBuf,
    doc_channel: tachyonix::Sender<TextSource>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_io()
        .enable_time()
        .thread_name("http server")
        .build()
        .expect("Couldn't build server");
    runtime
        .block_on(rpc_server(addr, db_path, doc_channel))
        .expect("Server failed");
}
