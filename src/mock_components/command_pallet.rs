//! Command Pallet is a "mock component" that helps to quickly setup keystroke navigation
//! for each screen including execution of actions with togglable flags.

use tuirealm::{
    command::{Cmd, CmdResult},
    tui::prelude::Rect,
    AttrValue, Attribute, Frame, MockComponent, State,
};

pub(crate) struct CommandPallet {}

impl MockComponent for CommandPallet {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        todo!()
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        todo!()
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        todo!()
    }

    fn state(&self) -> State {
        todo!()
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        todo!()
    }
}
