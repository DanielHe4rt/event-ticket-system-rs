# Event Ticket System

A heavy read application demo using monstrously awesome events as our payload!

## Core Concept

In this project, we're working with the idea of creating all the rows related to seats in a specific venue before sending it to the public. So, if the venue that your event will take place has 50k seats, you be creating 50k tickets.

Also, to avoid race condition for the ones that will be buying the tickets, we're gonna be using CDC (Change Data Capture) to make sure that when someone start the checkout process, no one will be able to purchase that specific seat/ticket.

Imagine that you have 15 minutes to finish the purchase. When you start it, a row with your `session_id, event_id and ticket_id` will be inserted at `order_queue` table which has:

```cql
CREATE TABLE order_queue (
	event_id uuid,
	ticket_id uuid,
	started_at timestamp,
	session_id text,
	PRIMARY (event_id, ticket_id, started_at)
) WITH cdc = {'enabled': 'true'} AND default_time_to_live = 60 * 15
```

If someone starts to purchase and stop in the middle of it, we can track them because for every operation related to the `order_queue`, a new log gonna be created

```text
cqlsh:ks> select * from order_queue_scylla_cdc_log  LIMIT 1;

cdc$stream_id                      | cdc$time                             | cdc$batch_seq_no | cdc$deleted_session_id | cdc$end_of_batch | cdc$operation | cdc$ttl | event_id                             | session_id                           | started_at                      | ticket_id
------------------------------------+--------------------------------------+------------------+------------------------+------------------+---------------+---------+--------------------------------------+--------------------------------------+---------------------------------+--------------------------------------
 0x8a78ebc0e79835301f196f14180001f1 | d907b072-48d1-11ef-5dc7-708fddfabcab |                0 |                   null |             True |             2 |       5 | a00924e9-0009-4563-90b7-a294d5812ada | 2eff47b7-7877-4189-bc57-281e529d9bc8 | 2024-07-23 08:59:19.829000+0000 | 69acda0c-be3f-435b-8adf-6b723a08d4f4

```

Using the crate [scylla-cdc-rust](https://github.com/scylladb/scylla-cdc-rust/) it gives us a way better DX for using this feature:

```rust
impl Consumer for OrderQueueConsumer {  
    async fn consume_cdc(&mut self, mut data: CDCRow<'_>) -> anyhow::Result<()> {  
        println!("Stream ID: {:}", data.stream_id);
        println!("TTL: {:}", data.ttl);

		let ticket_id = data.take_value("ticket_id").unwrap().as_uuid().unwrap();
	}
}
```

### Tables

* **venue_sections:** stores information about the event venue, pricing per seat and amount of seats from a specific section
* **events:** information about the events happening around
* **available_events [mv]:** retrieves only events that still have tickets to sell
* **tickets**: holds all the tickets which is created after a new event set up.
* **available_tickets [mv]:** retrieves only tickets with status 'available'
* **orders:** stores user attempts of purchase
* **order_queue:** stores the user and session with time limit for the purchase being accomplished.
* **order_queue_scylla_cdc_log:** receives all changes on order_queue (observer table).


### Data Modeling

```cql
CREATE KEYSPACE ks WITH replication = {'class': 'NetworkTopologyStrategy', 'datacenter1': '3'}  AND durable_writes = true;

CREATE TABLE ks.tickets (
    event_id uuid,
    ticket_id uuid,
    price decimal,
    section uuid,
    status text,
    type_id text,
    PRIMARY KEY (event_id, ticket_id)
) WITH CLUSTERING ORDER BY (ticket_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE MATERIALIZED VIEW ks.available_tickets AS
    SELECT event_id, status, ticket_id, price, section, type_id
    FROM ks.tickets
    WHERE event_id IS NOT null AND status IS NOT null AND ticket_id IS NOT null
    PRIMARY KEY ((event_id, status), ticket_id)
    WITH CLUSTERING ORDER BY (ticket_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE TABLE ks.order_queue (
    event_id uuid,
    ticket_id uuid,
    started_at timestamp,
    session_id text,
    PRIMARY KEY (event_id, ticket_id, started_at)
) WITH CLUSTERING ORDER BY (ticket_id ASC, started_at ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 5
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE TABLE ks.venue_sections (
    venue_id uuid,
    section_id uuid,
    type_id text,
    price decimal,
    seats_count int,
    PRIMARY KEY (venue_id, section_id, type_id)
) WITH CLUSTERING ORDER BY (section_id ASC, type_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE TABLE ks.orders (
    order_id uuid PRIMARY KEY,
    event_id uuid,
    purchased_at timestamp,
    status text,
    total_price decimal,
    user_id uuid
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE TABLE ks.order_queue_scylla_cdc_log (
    "cdc$stream_id" blob,
    "cdc$time" timeuuid,
    "cdc$batch_seq_no" int,
    "cdc$deleted_session_id" boolean,
    "cdc$end_of_batch" boolean,
    "cdc$operation" tinyint,
    "cdc$ttl" bigint,
    event_id uuid,
    session_id text,
    started_at timestamp,
    ticket_id uuid,
    PRIMARY KEY ("cdc$stream_id", "cdc$time", "cdc$batch_seq_no")
) WITH CLUSTERING ORDER BY ("cdc$time" ASC, "cdc$batch_seq_no" ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'enabled': 'false', 'keys': 'NONE', 'rows_per_partition': 'NONE'}
    AND comment = 'CDC log for ks.order_queue'
    AND compaction = {'class': 'TimeWindowCompactionStrategy', 'compaction_window_size': '60', 'compaction_window_unit': 'MINUTES', 'expired_sstable_check_frequency_seconds': '1800'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 0
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE TABLE ks.events (
    event_id uuid PRIMARY KEY,
    date timestamp,
    event_type text,
    has_tickets boolean,
    name text,
    slug text,
    venue_id uuid
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';

CREATE MATERIALIZED VIEW ks.available_events AS
    SELECT has_tickets, event_id, date, event_type, name, slug, venue_id
    FROM ks.events
    WHERE has_tickets IS NOT null AND event_id IS NOT null
    PRIMARY KEY (has_tickets, event_id)
    WITH CLUSTERING ORDER BY (event_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'SizeTieredCompactionStrategy'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1.0
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE';
```

### INSERT Queries

#### 1. Insert into `tickets`
```sql
INSERT INTO tickets (event_id, ticket_id, price, section, status, type_id) VALUES (?, ?, ?, ?, ?, ?);
```

#### 2. Insert into `order_queue`
```sql
INSERT INTO order_queue (event_id, ticket_id, started_at, session_id) VALUES (?, ?, ?, ?);
```

#### 3. Insert into `venue_sections`
```sql
INSERT INTO venue_sections (venue_id, section_id, type_id, price, seats_count) VALUES (?, ?, ?, ?, ?);
```

#### 4. Insert into `orders`
```sql
INSERT INTO orders (order_id, event_id, purchased_at, status, total_price, user_id) VALUES (?, ?, ?, ?, ?, ?);
```


#### 5. Insert into `events`
```sql
INSERT INTO events (event_id, date, event_type, has_tickets, name, slug, venue_id) VALUES (?, ?, ?, ?, ?, ?, ?);
```

### SELECT Queries

#### 1. Select from `tickets`
```sql
SELECT event_id, status, ticket_id, price, section, type_id FROM tickets WHERE event_id = ? AND status IS NOT null AND ticket_id IS NOT null;
```

#### 2. Select from `order_queue`
```sql
SELECT event_id, ticket_id, started_at, session_id FROM order_queue WHERE event_id = ? AND ticket_id = ? AND started_at = ?;
```

#### 3. Select from `venue_sections`
```sql
SELECT * FROM venue_sections WHERE venue_id = ? AND section_id = ? AND type_id = ?;
```

#### 4. Select from `orders`
```sql
SELECT * FROM orders WHERE order_id = ?;
```

#### 5. Select from `events`
```sql
SELECT * FROM events WHERE event_id IS NOT null AND has_tickets IS NOT null;
```

#### 6. Select from `available_tickets`
```sql
SELECT * FROM available_tickets WHERE event_id IS NOT null AND status IS NOT null AND ticket_id IS NOT null;
```

#### 7. Select from `available_events`
```sql
SELECT * FROM available_events WHERE has_tickets = true AND event_id IS NOT null;
```

These are the possible `INSERT` and `SELECT` queries based on the provided schema.