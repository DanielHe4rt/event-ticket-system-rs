use charybdis::macros::{charybdis_model, charybdis_view_model};
use charybdis::types::{Boolean, Text, Timestamp, Uuid};
use chrono::Utc;
use rand::prelude::SliceRandom;

#[charybdis_model(
    table_name = events,
    partition_keys = [event_id],
    clustering_keys = [],
    global_secondary_indexes = [],
    local_secondary_indexes = [],
    static_columns = []
)]
pub struct Event {
    pub event_id: Uuid,
    pub event_type: Text,
    pub name: Text,
    pub slug: Text,
    pub date: Timestamp,
    pub venue_id: Uuid,
    pub has_tickets: Boolean
}

#[charybdis_view_model(
    table_name=available_events,
    base_table=events,
    partition_keys=[has_tickets],
    clustering_keys=[event_id]
)]
pub struct AvailableEvent {
    pub event_id: Uuid,
    pub event_type: Text,
    pub name: Text,
    pub slug: Text,
    pub date: Timestamp,
    pub venue_id: Uuid,
    pub has_tickets: Boolean
}

impl Event {
    pub fn random(venue_id: Uuid) -> Self {
        let generate_event_name = |event_type: &str| {
            let mut rng = rand::thread_rng();
            let cool_event_names = vec![
                "The Beatles",
                "Queen",
                "The Rolling Stones",
            ];
            let cool_event_name = cool_event_names.choose(&mut rng).unwrap();

            format!("{} event {}", cool_event_name, event_type)
        };

        let event_types = vec!["music", "sports", "theater"];
        let event_type = event_types.choose(&mut rand::thread_rng()).unwrap();

        let event_name = generate_event_name(event_type);
        let event_slug = event_name.replace(" ", "-").to_lowercase().clone();

        Self {
            event_id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            name: event_name.to_string(),
            slug: event_slug,
            date: Utc::now(),
            venue_id,
            has_tickets: true
        }
    }
}