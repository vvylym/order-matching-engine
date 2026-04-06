#![allow(missing_docs)]
#![allow(clippy::type_complexity)]

//! In-memory event sink for harness tests and benches.

use crate::events::{Event, EventSink, EventSinkError};
use std::cell::RefCell;
use std::rc::Rc;

type EventBuffer = Rc<RefCell<Vec<Event>>>;

/// Sink that appends all events to a shared buffer (cloneable).
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
