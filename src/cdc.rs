use anyhow::Error;
use async_trait::async_trait;
use charybdis::operations::{Find, Insert};
use chrono::{DateTime, Utc};
use log::debug;
use scylla::frame::value::CqlTimeuuid;
use scylla::{CachingSession, Session};
use scylla_cdc::consumer::{CDCRow, Consumer, ConsumerFactory};
use scylla_cdc::log_reader::CDCLogReaderBuilder;
use std::ops::Add;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use crate::models::ticket::Ticket;

struct OrderQueueConsumer {
    session: Arc<CachingSession>,
}

#[async_trait]
impl Consumer for OrderQueueConsumer {
    async fn consume_cdc(&mut self, mut data: CDCRow<'_>) -> anyhow::Result<()> {
        let (secs, nanos) = data.time.get_timestamp().unwrap().to_unix();
        let cdc_timestamp = DateTime::from_timestamp(secs as i64, nanos).unwrap();

        let ttl = 5;

        let is_already_processed = cdc_timestamp.add(chrono::Duration::seconds(ttl)) < Utc::now();

        if !is_already_processed {
            return Ok(());
        }

        let mut ticket = Ticket {
            ticket_id: data.take_value("ticket_id").unwrap().as_uuid().unwrap(),
            event_id: data.take_value("event_id").unwrap().as_uuid().unwrap(),
            ..Default::default()
        };

        let processing_ticket = ticket
            .maybe_find_by_primary_key()
            .execute(&self.session)
            .await?;

        if processing_ticket.is_none() {
            delete_cdc_row(&self.session, data).await?;
            return Ok(());
        }

        let processing_ticket = processing_ticket.unwrap();
        if processing_ticket.status != "processing" {
            delete_cdc_row(&self.session, data).await?;
            return Ok(());
        }

        debug!("Purchase canceled due time limit: {:?}", ticket.ticket_id);

        ticket.status = "available".to_string();
        ticket.insert().execute(&self.session).await?;

        delete_cdc_row(&self.session, data).await?;

        Ok(())
    }
}

async fn delete_cdc_row(session: &Arc<CachingSession>, data: CDCRow<'_>) -> Result<(), Error> {
    session.execute("DELETE FROM order_queue_scylla_cdc_log WHERE \"cdc$stream_id\" = ? AND \"cdc$time\" = ? AND \"cdc$batch_seq_no\" = ?", (
        data.stream_id,
        CqlTimeuuid::from(data.time),
        data.batch_seq_no,
    )).await?;
    Ok(())
}
struct OrderQueueConsumerFactory {
    session: Arc<CachingSession>,
}

#[async_trait]
impl ConsumerFactory for OrderQueueConsumerFactory {
    async fn new_consumer(&self) -> Box<dyn Consumer> {
        Box::new(OrderQueueConsumer {
            session: self.session.clone(),
        })
    }
}

#[allow(unused)] // TODO: remove me when used
struct CountingConsumer {
    counter: Arc<AtomicUsize>,
}

#[async_trait]
impl Consumer for CountingConsumer {
    async fn consume_cdc(&mut self, _: CDCRow<'_>) -> anyhow::Result<()> {
        let curr = self.counter.fetch_add(1, Ordering::SeqCst);
        println!("Row no.{}", curr + 1);
        Ok(())
    }
}

pub async fn start_cdc_worker(
    regular_session: Arc<Session>,
    consumer_session: Arc<CachingSession>,
) -> anyhow::Result<()> {
    let factory = Arc::new(OrderQueueConsumerFactory {
        session: consumer_session.clone(),
    });

    loop {
        let end = chrono::Duration::from_std(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        )
        .unwrap();
        let start = end - chrono::Duration::seconds(30);

        let (_, handle) = CDCLogReaderBuilder::new()
            .session(regular_session.clone())
            .keyspace("ks")
            .table_name("order_queue")
            .start_timestamp(start)
            .end_timestamp(end)
            .safety_interval(chrono::Duration::seconds(1).to_std().unwrap())
            .sleep_interval(chrono::Duration::seconds(1).to_std().unwrap())
            .consumer_factory(factory.clone())
            .build()
            .await
            .expect("Creating the log reader failed!");

        handle.await?;
    }

    // NOTE: This code is unreachable, but it's here to show the loop that will run forever
}

