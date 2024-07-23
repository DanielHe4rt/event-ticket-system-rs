use std::str::FromStr;
use charybdis::macros::{charybdis_model, charybdis_view_model};
use charybdis::types::{Decimal, Text, Uuid};


#[charybdis_model(
    table_name = tickets,
    partition_keys = [event_id],
    clustering_keys = [ticket_id],
    global_secondary_indexes = [],
    local_secondary_indexes = []
)]
#[derive(Debug, Default)]
pub struct Ticket {
    pub ticket_id: Uuid,
    pub event_id: Uuid,
    pub type_id: Text,
    pub section: Uuid,
    pub price: Decimal,
    pub status: Text,
}

impl Ticket {
    pub fn from_event(event_id: Uuid, section_id: Uuid, price: Decimal) -> Self {
        Self {
            ticket_id: Uuid::new_v4(),
            event_id,
            type_id: "regular".to_string(),
            section: section_id,
            price,
            status: "available".to_string(),
        }
    }
}

#[charybdis_view_model(
    table_name=available_tickets,
    base_table=tickets,
    partition_keys=[event_id, status],
    clustering_keys=[ticket_id]
)]
pub struct AvailableTicket {
    pub ticket_id: Uuid,
    pub event_id: Uuid,
    pub type_id: Text,
    pub section: Uuid,
    pub price: Decimal,
    pub status: Text,
}