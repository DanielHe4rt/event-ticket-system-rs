use std::sync::Arc;

use charybdis::operations::Insert;
use chrono::Utc;
use log::debug;
use rand::prelude::SliceRandom;
use scylla::CachingSession;
use uuid::Uuid;
use crate::models::event::{AvailableEvent, Event};
use crate::models::order::{Order, OrderQueue};
use crate::models::ticket::{AvailableTicket, Ticket};
use crate::models::venue::VenueSection;

pub async fn handle_events(session: Arc<CachingSession>) -> anyhow::Result<()> {
    let venue = VenueSection::random();
    let event = Event::random(venue.venue_id.clone());

    // prepare event
    venue.insert().execute(&session).await?;
    event.insert().execute(&session).await?;

    debug!("Setting up event: {:?}", event.slug);
    debug!("Setting up venue: {:?}", venue.section_id);
    debug!("Setting up tickets: {:?}", venue.seats_count);

    for _ in 0..venue.seats_count {
        let ticket = Ticket::from_event(
            event.event_id,
            venue.section_id.clone(),
            venue.price.clone(),
        );
        ticket.insert().execute(&session).await?;
    }
    debug!("Tickets settled!");

    // Running the purchases
    loop {
        let result = AvailableEvent::find_by_has_tickets(true)
            .execute(&session)
            .await?
            .try_collect()
            .await?;

        let random_event = result.choose(&mut rand::thread_rng()).expect("No events available");

        let mut available_tickets = AvailableTicket::find_by_event_id_and_status(random_event.event_id, "available".to_string())
            .page_size(5)
            .execute(&session)
            .await?
            .try_collect()
            .await?;

        let available_ticket = available_tickets.choose(&mut rand::thread_rng()).unwrap();
        let mut ticket = Ticket {
            ticket_id: available_ticket.ticket_id,
            event_id: available_ticket.event_id.clone(),
            type_id: available_ticket.type_id.clone(),
            section: available_ticket.section.clone(),
            price: available_ticket.price.clone(),
            status: "processing".to_string(),
        };

        ticket.insert().execute(&session).await?;

        let order_queue = OrderQueue {
            event_id: random_event.event_id,
            ticket_id: available_ticket.ticket_id,
            session_id: Uuid::new_v4().to_string(),
            started_at: Utc::now(),
        };

        order_queue.insert().execute(&session).await?;

        let mut order = Order {
            order_id: Uuid::new_v4(),
            event_id: random_event.event_id,
            user_id: Uuid::new_v4(),
            status: "pending".to_string(),
            total_price: available_ticket.price.clone(),
            purchased_at: Utc::now(),
        };

        order.insert().execute(&session).await?;

        if rand::random::<f64>() > 0.3 {
            order.status = "failed".to_string();
            order.insert().execute(&session).await?;
            ticket.status = "available".to_string();
            ticket.insert().execute(&session).await?;
            debug!("Order failed: {:?}", order.order_id);
        } else if rand::random::<f64>() > 0.5 && rand::random::<f64>() < 0.7 {
            order.status = "completed".to_string();
            order.insert().execute(&session).await?;
            ticket.status = "taken".to_string();
            ticket.insert().execute(&session).await?;
            debug!("Order completed: {:?}", order.order_id);
        } else {
            debug!("Order pending: {:?}", order.order_id);
        }
    }

    // how do i get all queries that i have executed?



    Ok(())
}