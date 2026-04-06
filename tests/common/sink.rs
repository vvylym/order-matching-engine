//! Event sink that collects events for test assertions.

use omer::events::{Event, EventSink, EventSinkError};
use std::cell::RefCell;
use std::rc::Rc;

/// Shared event buffer (reduces type complexity for clippy).
type EventBuffer = Rc<RefCell<Vec<Event>>>;

/// Sink that appends all events to a shared buffer (cloneable for test access).
#[derive(Clone)]
pub struct CollectingEventSink {
    events: EventBuffer,
}

impl Default for CollectingEventSink {
    fn default() -> Self {
        Self {
            events: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl CollectingEventSink {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> std::cell::Ref<'_, Vec<Event>> {
        self.events.borrow()
    }

    /// Clear collected events (test helper).
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.events.borrow_mut().clear();
    }
}

impl EventSink for CollectingEventSink {
    fn emit(&self, event: Event) -> Result<(), EventSinkError> {
        self.events.borrow_mut().push(event);
        Ok(())
    }
}
