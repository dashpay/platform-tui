//! View for fetched documents navigation and inspection.

use std::collections::BTreeMap;

use dpp::{document::Document, platform_value::string_encoding::Encoding, prelude::Identifier};
use tuirealm::{
    props::{BorderSides, Borders, Color, TextSpan},
    tui::prelude::{Constraint, Direction, Layout, Rect},
    AttrValue, Attribute, Frame, MockComponent,
};

use crate::{
    backend::as_toml,
    ui::screen::{
        widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 5] = [
    ScreenCommandKey::new("q", "Back to Contracts"),
    ScreenCommandKey::new("C-n", "Next document"),
    ScreenCommandKey::new("C-p", "Prev document"),
    ScreenCommandKey::new("↓", "Scroll up down"),
    ScreenCommandKey::new("↑", "Scroll doc up"),
];

pub(crate) struct DocumentsQuerysetScreenController {
    current_batch: BTreeMap<Identifier, Option<Document>>,
    document_select: tui_realm_stdlib::List,
    document_view: Info,
}

impl DocumentsQuerysetScreenController {
    pub(crate) fn new(current_batch: BTreeMap<Identifier, Option<Document>>) -> Self {
        let mut document_select = tui_realm_stdlib::List::default()
            .rows(
                current_batch
                    .keys()
                    .map(|v| vec![TextSpan::new(v.to_string(Encoding::Base58))])
                    .collect(),
            )
            .borders(
                Borders::default()
                    .sides(BorderSides::LEFT | BorderSides::TOP | BorderSides::BOTTOM),
            )
            .selected_line(0)
            .highlighted_color(Color::Magenta);
        document_select.attr(Attribute::Scroll, AttrValue::Flag(true));
        document_select.attr(Attribute::Focus, AttrValue::Flag(true));

        let document_view = Info::new_scrollable(
            &current_batch
                .first_key_value()
                .map(|(_, v)| as_toml(v))
                .unwrap_or_else(String::new),
        );

        DocumentsQuerysetScreenController {
            current_batch,
            document_select,
            document_view,
        }
    }
}

impl ScreenController for DocumentsQuerysetScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Max(40), Constraint::Min(1)].as_ref())
            .split(area);

        self.document_select.view(frame, layout[0]);
        self.document_view.view(frame, layout[1]);
    }

    fn name(&self) -> &'static str {
        "Documents queryset"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        todo!()
    }
}
