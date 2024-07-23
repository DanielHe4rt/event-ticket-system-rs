use charybdis::macros::charybdis_model;
use charybdis::types::{Decimal, Text, Timestamp, Uuid};

const TTL: &str = "5";
#[charybdis_model(
    table_name = "orders",
    partition_keys = [order_id],
    clustering_keys = [],
    global_secondary_indexes = [],
    local_secondary_indexes = [],
    static_columns = []
)]
pub struct Order {
    pub order_id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub status: Text,
    pub total_price: Decimal,
    pub purchased_at: Timestamp,
}

#[charybdis_model(
    table_name = order_queue,
    partition_keys = [event_id],
    clustering_keys = [ticket_id, started_at],
    global_secondary_indexes = [],
    local_secondary_indexes = [],
    table_options = r"
         cdc = {'enabled': 'true'} AND default_time_to_live = 5
    "
)]
pub struct OrderQueue {
    pub event_id: Uuid,
    pub ticket_id: Uuid,
    pub session_id: Text,
    pub started_at: Timestamp,
}