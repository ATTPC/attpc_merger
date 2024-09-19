use super::error::EventBuilderError;
use super::event::Event;
use super::graw_frame::GrawFrame;
use super::pad_map::PadMap;

/// EventBuilder takes GrawFrames and composes them into Events.
///
/// The EventBuilder recieves data from the Merger and constructs an Event struct. The
/// Event struct can then be sent to an HDFWriter to write merged events to disk.
#[derive(Debug)]
pub struct EventBuilder {
    current_event_id: u32,
    pad_map: PadMap,
    frame_stack: Vec<GrawFrame>,
}

impl EventBuilder {
    /// Create a new EventBuilder.
    ///
    /// Requires a PadMap
    pub fn new(pad_map: PadMap) -> Self {
        EventBuilder {
            current_event_id: 0,
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
    pub fn append_frame(&mut self, frame: GrawFrame) -> Result<Option<Event>, EventBuilderError> {
        if frame.header.event_id > self.current_event_id && self.current_event_id != 0 {
            //event completed and start a new event.
            let event: Event = Event::new(&self.pad_map, &self.frame_stack)?;
            self.frame_stack.clear();
            self.current_event_id = frame.header.event_id;
            self.frame_stack.push(frame);

            Ok(Some(event))
        } else if self.current_event_id == 0 {
            // this is the first frame ever
            self.current_event_id = frame.header.event_id;
            self.frame_stack.push(frame);
            Ok(None)
        } else if frame.header.event_id < self.current_event_id {
            //Oops out of order
            Err(EventBuilderError::EventOutOfOrder(
                frame.header.event_id,
                self.current_event_id,
            ))
        } else {
            //Still building
            self.frame_stack.push(frame);
            Ok(None)
        }
    }

    /// Takes any remaining frames and flushes them to an event.
    ///
    /// Used at the end of processing a run.
    /// Returns None if there were no frames left over.
    pub fn flush_final_event(&mut self) -> Option<Event> {
        if self.frame_stack.is_empty() {
            match Event::new(&self.pad_map, &self.frame_stack) {
                Ok(event) => Some(event),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}
