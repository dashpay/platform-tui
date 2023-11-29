//! Platform invo views.


use std::fmt::{self, Display};

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent},
    ui::{
        form::{Input, InputStatus, SelectInput},
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};
use crate::backend::platform_info::PlatformInfoTask::FetchCurrentEpochInfo;
use crate::backend::{StrategyTask, Task};

const COMMAND_KEYS: [ScreenCommandKey; 1] = [
    ScreenCommandKey::new("i", "Fetch recent Platform information"),
];

pub(crate) struct PlatformInfoScreenController {
    info: Info,
}

impl_builder!(PlatformInfoScreenController);


impl PlatformInfoScreenController {
    pub(crate) async fn new(_app_state: &AppState) -> Self {
        PlatformInfoScreenController {
            info: Info::new_fixed("Identity management commands"),
        }
    }
}

impl ScreenController for PlatformInfoScreenController {
    fn name(&self) -> &'static str {
        "Platform Information"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                           code: Key::Char('q'),
                           modifiers: KeyModifiers::NONE,
                       }) => ScreenFeedback::PreviousScreen,

            Event::Key(KeyEvent {
                           code: Key::Char('i'),
                           modifiers: KeyModifiers::NONE,
                       }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentEpochInfo),
                block: true,
            },

            Event::Backend(BackendEvent::TaskCompleted {
                               task: Task::PlatformInfo(_),
                               execution_result,
                           }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }
}
