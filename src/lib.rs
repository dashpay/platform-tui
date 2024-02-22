pub mod backend;
pub mod config;
pub mod ui;

use backend::BackendEvent;
use tuirealm::event::KeyEvent;

pub enum Event<'s> {
    Key(KeyEvent),
    Backend(BackendEvent<'s>),
    RedrawDebounceTimeout,
}
