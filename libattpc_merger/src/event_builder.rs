use super::channel_map::GetChannelMap;
use super::error::EventBuilderError;
use super::event::Event;
use super::graw_frame::GrawFrame;

/// EventBuilder takes GrawFrames and composes them into Events.
///
/// The EventBuilder recieves data from the Merger and constructs an Event struct. The
/// Event struct can then be sent to an HDFWriter to write merged events to disk.
#[derive(Debug)]
pub struct EventBuilder {
    current_event_id: Option<u32>,
    pad_map: GetChannelMap,
    frame_stack: Vec<GrawFrame>,
}

impl EventBuilder {
    /// Create a new EventBuilder.
    ///
    /// Requires a GetChannelMap
    pub fn new(pad_map: GetChannelMap) -> Self {
        EventBuilder {
            current_event_id: None,
            pad_map,
            frame_stack: Vec::new(),
        }
    }

    /// Add a frame to the event.
    ///
    /// If the frame does not have the same EventID as the event currently being built,
    /// this is taken as indication that that event is complete, and a new event should be started for the frame given.
    /// Returns a `Result<Option<Event>>`. If the Option is None, the event being built is not complete. If the Optiion is Some,
    /// the event being built was completed, and a new event was started for the frame that was passed in.
    #[allow(clippy::comparison_chain)]
    pub fn append_frame(&mut self, frame: GrawFrame) -> Result<Option<Event>, EventBuilderError> {
        if let Some(current_id) = self.current_event_id {
            if frame.header.event_id < current_id {
                // Some how we recieved a frame from a past event
                Err(EventBuilderError::EventOutOfOrder(
                    frame.header.event_id,
                    current_id,
                ))
            } else if frame.header.event_id > current_id {
                // We recieved a frame from the next event; emit the built event and start a new one
                let event = Event::new(&self.pad_map, &self.frame_stack)?;
                self.frame_stack.clear();
                self.current_event_id = Some(frame.header.event_id);
                self.frame_stack.push(frame);
                Ok(Some(event))
            } else {
                // We recieved a frame for this event
                self.frame_stack.push(frame);
                Ok(None)
            }
        } else {
            // This is the first frame ever in history
            self.current_event_id = Some(frame.header.event_id);
            self.frame_stack.push(frame);
            Ok(None)
        }
    }

    /// Takes any remaining frames and flushes them to an event.
    ///
    /// Used at the end of processing a run.
    /// Returns None if there were no frames left over.
    pub fn flush_final_event(&mut self) -> Option<Event> {
        if !self.frame_stack.is_empty() {
            match Event::new(&self.pad_map, &self.frame_stack) {
                Ok(event) => Some(event),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}
