//! Contract fetching screen module.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::ContractsScreenController;
use crate::{
    backend::{BackendEvent, ContractTask, Task},
    ui::screen::{
        utils::impl_builder_no_args, widgets::info::Info, ScreenCommandKey, ScreenController,
        ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 3] = [
    ScreenCommandKey::new("q", "Back to Contracts"),
    ScreenCommandKey::new("p", "Fetch Dashpay contract"),
    ScreenCommandKey::new("n", "Fetch DPNS contract"),
];

pub(crate) struct FetchSystemContractScreenController {
    info: Info,
}

impl_builder_no_args!(FetchSystemContractScreenController);

impl FetchSystemContractScreenController {
    pub(crate) fn new() -> Self {
        Self {
            info: Info::new_fixed("Fetch system contracts"),
        }
    }
}

impl ScreenController for FetchSystemContractScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }

    fn name(&self) -> &'static str {
        "System Contracts"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen,

            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Contract(ContractTask::FetchDashpayContract),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Contract(ContractTask::FetchDPNSContract),
                block: true,
            },

            Event::Backend(
                BackendEvent::TaskCompleted {
                    task: Task::Contract(_),
                    execution_result,
                }
                | BackendEvent::TaskCompletedStateChange {
                    task: Task::Contract(_),
                    execution_result,
                    ..
                },
            ) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}
