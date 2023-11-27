//! View for fetched documents navigation and inspection.

use std::collections::BTreeMap;

use dpp::{document::Document, prelude::Identifier};
use tuirealm::{tui::prelude::Rect, Frame};

use crate::{
    ui::screen::{ScreenCommandKey, ScreenController, ScreenFeedback, ScreenToggleKey},
    Event,
};

pub(crate) struct DocumentsQuerysetScreenController {
    current_batch: BTreeMap<Identifier, Option<Document>>,
}

impl DocumentsQuerysetScreenController {
    pub(crate) fn new(current_batch: BTreeMap<Identifier, Option<Document>>) -> Self {
        DocumentsQuerysetScreenController { current_batch }
    }
}

impl ScreenController for DocumentsQuerysetScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        todo!()
    }

    fn name(&self) -> &'static str {
        "Documents queryset"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        todo!()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        todo!()
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        todo!()
    }
}
