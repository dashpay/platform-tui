//! First screen of the application.

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
}
