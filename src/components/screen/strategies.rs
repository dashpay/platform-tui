

#[derive(MockComponent)]
pub(crate) struct StrategiesScreen {
    component: Paragraph,
}

impl StrategiesScreen {
    pub(crate) fn new() -> Self {
        StrategiesScreen {
            component: Paragraph::default()
                .text([TextSpan::new("Strategies management commands")].as_ref()),
        }
    }
}

impl Component<Message, NoUserEvent> for StrategiesScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct StrategiesScreenCommands {
    component: CommandPallet,
}

impl StrategiesScreenCommands {
    pub(crate) fn new() -> Self {
        StrategiesScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 's',
                    description: "Select a strategy",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Create a new strategy",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for WalletScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                                code: Key::Char('q'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                                code: Key::Char('s'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::NextScreen(Screen::SavedStrategies)),
            Event::Keyboard(KeyEvent {
                                code: Key::Char('c'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::NextScreen(Screen::CreateStrategy)),
            _ => None,
        }
    }
}