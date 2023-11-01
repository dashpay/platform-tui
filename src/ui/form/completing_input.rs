//! Completing input module.

use tui_realm_stdlib::{Input, List};

pub(super) struct CompletingInput {
    input: Input,
    variants: List,
    choosing_completion: bool,
}

// TODO
