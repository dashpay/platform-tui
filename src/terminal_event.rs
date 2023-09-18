use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};

use crate::app::AppResult;

/// Terminal events.
#[derive(Clone, Copy, Debug)]
pub enum TerminalEvent {
    /// Terminal tick.
    Tick,
    /// Key press.
    Key(KeyEvent),
    /// Mouse click/scroll.
    Mouse(MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
}

/// Terminal event handler.
#[allow(dead_code)]
#[derive(Debug)]
pub struct TerminalEventHandler {
    /// Event sender channel.
    sender: mpsc::Sender<TerminalEvent>,
    /// Event receiver channel.
    receiver: mpsc::Receiver<TerminalEvent>,
    /// Event handler thread.
    handler: thread::JoinHandle<()>,
}

impl TerminalEventHandler {
    /// Constructs a new instance of [`EventHandler`].
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || loop {
                if event::poll(Duration::from_secs(5)).expect("unable to poll for events") {
                    match event::read().expect("unable to read event") {
                        CrosstermEvent::Key(e) => sender.send(TerminalEvent::Key(e)),
                        CrosstermEvent::Resize(w, h) => sender.send(TerminalEvent::Resize(w, h)),
                        _ => Ok(()),
                    }
                    .expect("failed to send terminal event")
                }
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    /// Receive the next event from the handler thread.
    ///
    /// This function will always block the current thread if
    /// there is no data available and it's possible for more data to be sent.
    pub fn next(&self) -> AppResult<TerminalEvent> {
        Ok(self.receiver.recv()?)
    }
}
