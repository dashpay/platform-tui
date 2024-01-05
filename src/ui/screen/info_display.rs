use dpp::{
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0,
    },
    prelude::{Identity, IdentityPublicKey},
};

use crate::ui::IdentityBalance;

pub struct TabbedString {
    pub indent: usize,
    pub content: String,
}

impl TabbedString {
    const SPACES_PER_INDENT: usize = 2;

    pub fn new(indent: usize, content: &str) -> Self {
        TabbedString {
            indent,
            content: content.into(),
        }
    }

    pub fn to_string(&self, parent_indent: usize) -> String {
        format!(
            "{:indent$}{}",
            "",
            self.content,
            indent = (parent_indent + self.indent) * Self::SPACES_PER_INDENT
        )
    }
}

macro_rules! tabbed_string {
    ($indent:expr, $content:expr) => {
        TabbedString {
            indent: $indent,
            content: $content.to_string(),
        }
    };
}

macro_rules! tabbed_key_value_string {
    ($indent:expr, $key:expr, $value:expr) => {
        TabbedString {
            indent: $indent,
            content: format!("{}: {}", $key, $value),
        }
    };
}

macro_rules! tabbed_key_value_display_info_string {
    ($indent:expr, $key:expr, $value:expr) => {
        TabbedString {
            indent: $indent,
            content: format!("{}: {}", $key, $value.display_info($indent + 1)),
        }
    };
}

macro_rules! tabbed_key_value_iter_string {
    ($indent:expr, $key:expr, $value:expr) => {
        TabbedString {
            indent: $indent,
            content: format!(
                "{}: {}",
                $key,
                $value
                    .into_iter()
                    .map(
                        |(key, value)| tabbed_key_value_display_info_string!($indent, key, value)
                            .to_string($indent)
                    )
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
        }
    };
}

pub trait InfoDisplay {
    fn display_info_lines(&self) -> Vec<TabbedString>;

    fn display_info(&self, parent_indent: usize) -> String {
        self.display_info_lines()
            .into_iter()
            .map(|tabbed_string| tabbed_string.to_string(parent_indent))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl InfoDisplay for Identity {
    fn display_info_lines(&self) -> Vec<TabbedString> {
        vec![
            tabbed_string!(0, "Identity"),
            tabbed_key_value_string!(1, "Id", self.id()),
            tabbed_key_value_string!(
                1,
                "Balance",
                IdentityBalance::from_credits(self.balance()).dash_str()
            ),
            tabbed_key_value_string!(1, "Revision", self.revision()),
            tabbed_key_value_iter_string!(1, "Public Keys", self.public_keys()),
        ]
    }
}

impl InfoDisplay for IdentityPublicKey {
    fn display_info_lines(&self) -> Vec<TabbedString> {
        vec![tabbed_string!(0, format!("{} Key", self.key_type()))]
    }
}
