use std::collections::HashMap;

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::calendar::{Calendar, Event};
use crate::db::{DeltaType, OverlayDelta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayEvent {
    pub date: String,
    pub start: String,
    pub end: String,
    pub title: String,
    pub location: Option<String>,
    pub organizer: Option<String>,
    pub description: Option<String>,
}

pub fn event_match_key(event: &Event) -> String {
    format!(
        "{}|{}-{}|{}",
        event.date.format("%Y-%m-%d"),
        event.start.format("%H:%M"),
        event.end.format("%H:%M"),
        event.title
    )
}

pub fn parse_overlay_event(json: &str) -> Option<OverlayEvent> {
    serde_json::from_str(json).ok()
}

pub fn overlay_event_to_event(overlay: &OverlayEvent) -> Option<Event> {
    let date = NaiveDate::parse_from_str(&overlay.date, "%Y-%m-%d").ok()?;
    let start = NaiveTime::parse_from_str(&overlay.start, "%H:%M").ok()?;
    let end = NaiveTime::parse_from_str(&overlay.end, "%H:%M").ok()?;

    Some(Event {
        date,
        start,
        end,
        title: overlay.title.clone(),
        location: overlay.location.clone(),
        organizer: overlay.organizer.clone(),
        description: overlay.description.clone(),
    })
}

pub fn merge_calendar(
    calendar: Calendar,
    deltas: &[OverlayDelta],
    tag: Option<&str>,
) -> Calendar {
    let mut delta_map: HashMap<&str, &OverlayDelta> = HashMap::new();
    let mut add_deltas: Vec<&OverlayDelta> = Vec::new();

    for delta in deltas {
        match delta.r#type {
            DeltaType::Delete | DeltaType::Modify => {
                if let Some(ref key) = delta.match_key {
                    delta_map.insert(key, delta);
                }
            }
            DeltaType::Add => {
                add_deltas.push(delta);
            }
        }
    }

    let mut merged_events = Vec::new();

    for event in calendar.events {
        let key = event_match_key(&event);
        match delta_map.get(key.as_str()) {
            Some(delta) => match delta.r#type {
                DeltaType::Delete => {
                    // Skip this event entirely
                }
                DeltaType::Modify => {
                    if let Some(ref event_json) = delta.event_json {
                        if let Some(overlay) = parse_overlay_event(event_json) {
                            if let Some(mut modified) = overlay_event_to_event(&overlay) {
                                if let Some(t) = tag {
                                    modified.title = format!("[{}] {}", t, modified.title);
                                }
                                merged_events.push(modified);
                            }
                        }
                    }
                }
                DeltaType::Add => unreachable!(),
            },
            None => {
                merged_events.push(event);
            }
        }
    }

    // Add new events
    for delta in add_deltas {
        if let Some(ref event_json) = delta.event_json {
            if let Some(overlay) = parse_overlay_event(event_json) {
                if let Some(mut event) = overlay_event_to_event(&overlay) {
                    if let Some(t) = tag {
                        event.title = format!("[{}] {}", t, event.title);
                    }
                    merged_events.push(event);
                }
            }
        }
    }

    Calendar {
        name: calendar.name,
        events: merged_events,
    }
}
