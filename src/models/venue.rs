use std::str::FromStr;

use charybdis::macros::charybdis_model;
use charybdis::types::{Decimal, Int, Text, Uuid};
use rand::Rng;

#[charybdis_model(
    table_name = "venue_sections",
    partition_keys = [venue_id],
    clustering_keys = [section_id, type_id],
    global_secondary_indexes = [],
    local_secondary_indexes = [],
    static_columns = []
)]
pub struct VenueSection {
    pub venue_id: Uuid,
    pub section_id: Uuid,
    pub seats_count: Int,
    pub type_id: Text,
    pub price: Decimal,
}

impl VenueSection {
    pub fn random () -> Self {
        let random_number_between = |min: i32, max: i32| {
            let mut rng = rand::thread_rng();
            rng.gen_range(min..max)
        };

        let random_price = |min: f64, max: f64| {
            let mut rng = rand::thread_rng();
            let random_price = rng.gen_range(min..max);
            Decimal::from_str(&format!("{:.2}", random_price)).unwrap()
        };
        Self {
            venue_id: Uuid::new_v4(),
            section_id: Uuid::new_v4(),
            seats_count: random_number_between(1_000, 10_000),
            type_id: "type_id".to_string(),
            price: random_price(10.0, 100.0)
        }
    }
}