//! Completion backends.

pub(crate) trait CompletionEngine {}

pub(crate) struct HistoryCompletionEngine {}

impl CompletionEngine for HistoryCompletionEngine {}
