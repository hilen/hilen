use std::sync::mpsc::{Receiver, channel};

use rapier3d::{
    geometry::{CollisionEvent, ContactForceEvent},
    prelude::ChannelEventCollector,
};

pub(crate) struct EventHandler {
    pub(crate) _contact:     Receiver<ContactForceEvent>,
    pub(crate) intersection: Receiver<CollisionEvent>,
    pub(crate) handler:      ChannelEventCollector,
}

impl Default for EventHandler {
    #[allow(clippy::used_underscore_binding)]
    fn default() -> Self {
        let (contact_send, _contact) = channel();
        let (intersection_send, intersection) = channel();
        // Nothing here makes a soft body. Rapier drops the events of a sender
        // whose receiver is gone.
        let tear_send = channel().0;
        let handler = ChannelEventCollector::new(intersection_send, contact_send, tear_send);
        Self {
            _contact,
            intersection,
            handler,
        }
    }
}
