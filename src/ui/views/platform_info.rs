//! Platform info views.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::backend::platform_info::PlatformInfoTask::{
    FetchCurrentValidatorSetInfo, FetchCurrentValidatorSetInfoAndShowQueue,
    FetchCurrentValidatorSetInfoAndShowReducedQueue, FetchCurrentWithdrawalsInQueue,
    FetchRecentlyCompletedWithdrawals, FetchTotalCreditsOnPlatform,
};

use crate::{
    backend::{
        platform_info::PlatformInfoTask::{
            FetchCurrentEpochInfo, FetchCurrentVersionVotingState, FetchSpecificEpochInfo,
        },
        AppState, BackendEvent, Task,
    },
    ui::{
        form::{
            parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus,
            TextInput,
        },
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 10] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("c", "Fetch current Platform epoch info"),
    ScreenCommandKey::new("m", "Fetch total credits on platform"),
    ScreenCommandKey::new("i", "Fetch previous Platform epoch info"),
    ScreenCommandKey::new("v", "Current version voting"),
    ScreenCommandKey::new("p", "Current proposer and validator set info"),
    ScreenCommandKey::new("k", "View Proposer Queues"),
    ScreenCommandKey::new("l", "View Reduced Proposer Queues"),
    ScreenCommandKey::new("w", "View Withdrawals in Queue"),
    ScreenCommandKey::new("e", "View Recently Completed Withdrawals"),
];

pub(crate) struct PlatformInfoScreenController {
    info: Info,
}

impl_builder!(PlatformInfoScreenController);

impl PlatformInfoScreenController {
    pub(crate) async fn new(_app_state: &AppState) -> Self {
        PlatformInfoScreenController {
            info: Info::new_scrollable("Platform info"),
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
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentEpochInfo),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('m'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchTotalCreditsOnPlatform),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('v'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentVersionVotingState),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentValidatorSetInfo),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentValidatorSetInfoAndShowQueue),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentValidatorSetInfoAndShowReducedQueue),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('w'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentWithdrawalsInQueue),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('e'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchRecentlyCompletedWithdrawals),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(EpochNumberChooserFormController::new())),

            // Forward event to upper part of the screen for scrolls and stuff
            Event::Key(k) => {
                if self.info.on_event(k) {
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
            }

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

struct EpochNumberChooserFormController {
    input: TextInput<DefaultTextInputParser<u16>>,
}

impl EpochNumberChooserFormController {
    fn new() -> Self {
        EpochNumberChooserFormController {
            input: TextInput::new("Epoch number"),
        }
    }
}

impl FormController for EpochNumberChooserFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(epoch) => FormStatus::Done {
                task: Task::PlatformInfo(FetchSpecificEpochInfo(epoch)),
                block: true,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Epoch number"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Input epoch number"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
