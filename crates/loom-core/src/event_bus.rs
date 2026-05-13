use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use uuid::Uuid;

use loom_types::api::SessionStreamEvent;

/// Per-session broadcast hub used by the daemon's SSE endpoint. Each session
/// gets its own [`broadcast::Sender`]; subscribers receive every event
/// published after they subscribed. The kernel publishes after every state
/// transition; the SSE handler subscribes and pumps the receiver to the
/// browser as `text/event-stream`.
#[derive(Debug, Default)]
pub struct SessionEventBus {
    inner: Mutex<HashMap<Uuid, broadcast::Sender<SessionStreamEvent>>>,
}

impl SessionEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, session_id: Uuid) -> broadcast::Receiver<SessionStreamEvent> {
        let mut map = self.inner.lock().expect("session event bus poisoned");
        map.entry(session_id)
            .or_insert_with(|| broadcast::channel(256).0)
            .subscribe()
    }

    pub fn publish(&self, session_id: Uuid, event: SessionStreamEvent) {
        let map = self.inner.lock().expect("session event bus poisoned");
        if let Some(sender) = map.get(&session_id) {
            // ignore: zero subscribers means no one cares about this event yet
            let _ = sender.send(event);
        }
    }

    pub fn subscriber_count(&self) -> usize {
        let map = self.inner.lock().expect("session event bus poisoned");
        map.values().map(|sender| sender.receiver_count()).sum()
    }

    pub fn heartbeat_event() -> SessionStreamEvent {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        SessionStreamEvent::Heartbeat { unix_ms }
    }
}
