use std::sync::Arc;
use pretty_env_logger::env_logger::{Builder, Env};
use scylla::{CachingSession, SessionBuilder};
use tokio::task::JoinSet;
use log::debug;

mod models;
mod cdc;
mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    //Builder::from_env(Env::default().default_filter_or("info")).init();
    debug!("Starting up");

    let regular_session = Arc::new(SessionBuilder::new()
        .known_node("127.0.0.1:9042")
        .build()
        .await?
    );

    let consumer_session = SessionBuilder::new()
        .known_node("127.0.0.1:9042")
        .build()
        .await?;

    consumer_session.use_keyspace("ks", true).await?;
    let cdc_session = Arc::new(CachingSession::from(consumer_session, 20));
    let worker_session = Arc::clone(&cdc_session);

    let mut set = JoinSet::new();

    set.spawn(async move {
        cdc::start_cdc_worker(regular_session, Arc::clone(&cdc_session)).await.unwrap();
    });

    set.spawn(async move {
        let db = Arc::clone(&worker_session);
        worker::handle_events(Arc::clone(&db)).await.unwrap();
    });

    while let Some(res) = set.join_next().await {
        let out = res?;
        // ...
    }

    Ok(())
}
