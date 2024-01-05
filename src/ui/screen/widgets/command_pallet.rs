//! Command pallet definitions.

use itertools::Itertools;
use tui_realm_stdlib::Table;
use tuirealm::{props::TextSpan, tui::prelude::Rect, Frame, MockComponent};

use crate::ui::screen::ScreenController;

const KEYS_PER_ROW: usize = 3;

pub(crate) fn view(frame: &mut Frame, area: Rect, controller: &impl ScreenController) {
    let commands = controller.command_keys().to_owned();
    let toggles = controller.toggle_keys().to_owned();

    let mut table_vec = Vec::new();

    for row in &commands
        .iter()
        .map(|c| TextSpan::new(format!("{} - {}", c.keybinding, c.description)))
        .chain(toggles.iter().map(|t| {
            let span = TextSpan::new(format!("{} - {}", t.keybinding, t.description)).italic();
            if t.toggle {
                span.bold()
            } else {
                span
            }
        }))
        .chunks(KEYS_PER_ROW)
    {
        let mut row_vec = Vec::new();
        for span in row {
            row_vec.push(span);
        }

        table_vec.push(row_vec);
    }

    Table::default().table(table_vec).view(frame, area);
}
