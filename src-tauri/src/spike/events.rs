// M0.5 SPIKE — in-memory measurement event store.
//
// Experiments push structured events here; the spike frontend pulls them
// through IPC. Nothing in this file is production infrastructure and
// nothing is persisted (measurements land in docs/M0.5-*.md manually).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeEvent {
    pub ts_unix_ms: u64,
    pub kind: String,
    pub detail: BTreeMap<String, String>,
}

impl SpikeEvent {
    pub fn new(kind: &str, detail: &[(&str, String)]) -> SpikeEvent {
        SpikeEvent {
            ts_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            kind: kind.to_string(),
            detail: detail
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
        }
    }
}

static EVENTS: Mutex<Vec<SpikeEvent>> = Mutex::new(Vec::new());

pub fn record(kind: &str, detail: &[(&str, String)]) {
    let event = SpikeEvent::new(kind, detail);
    // Also visible in the app log for post-mortem analysis.
    crate::diagnostics::debug(
        "spike.event",
        &[
            ("kind", kind),
            ("detail", &serde_json::json!(event.detail).to_string()),
        ],
    );
    if let Ok(mut events) = EVENTS.lock() {
        // Bound memory: the spike never needs more than a few thousand events.
        if events.len() > 10_000 {
            events.drain(0..5_000);
        }
        events.push(event);
    }
}

pub fn snapshot(kind_prefix: Option<&str>) -> Vec<SpikeEvent> {
    match EVENTS.lock() {
        Ok(events) => events
            .iter()
            .filter(|e| kind_prefix.is_none_or(|p| e.kind.starts_with(p)))
            .cloned()
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub fn clear() {
    if let Ok(mut events) = EVENTS.lock() {
        events.clear();
    }
}
