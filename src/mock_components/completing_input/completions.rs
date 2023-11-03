//! Completion backends.

use std::{ops::Deref, slice};

pub(crate) trait CompletionEngine {
    type Completion: Deref<Target = str>;

    type Completions<'a>: Iterator<Item = &'a Self::Completion>
    where
        Self: 'a;

    fn get_completions_list<'a>(&'a self, input: &str) -> Self::Completions<'a>;
}

#[derive(Debug, Default)]
pub(crate) struct HistoryCompletionEngine {
    history_items: Vec<String>,
}

impl CompletionEngine for HistoryCompletionEngine {
    type Completion = String;
    type Completions<'a> = slice::Iter<'a, String>;

    fn get_completions_list<'a>(&'a self, _input: &str) -> Self::Completions<'a> {
        self.history_items.iter()
    }
}

impl HistoryCompletionEngine {
    pub(crate) fn add_history_item(&mut self, item: String) {
        self.history_items.push(item);
    }
}
