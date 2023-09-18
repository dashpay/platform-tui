//! First screen of the application.

use super::Key;

#[derive(Debug, Clone)]
pub struct MainScreen {
    identity_private_key: Option<()>, // TODO
}

impl MainScreen {
    pub fn new() -> Self {
        MainScreen {
            identity_private_key: None,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        [
            Key {
                key: 'i',
                description: "Identities",
            },
            Key {
                key: 't',
                description: "Test1",
            },
            Key {
                key: 'o',
                description: "Test2",
            },
            Key {
                key: 'p',
                description: "Test3",
            },
            Key {
                key: 'q',
                description: "Test4",
            },
            Key {
                key: 'r',
                description: "Test4",
            },
            Key {
                key: 's',
                description: "Test5",
            },
            Key {
                key: 'u',
                description: "Test6",
            },
        ]
        .iter()
    }
}
