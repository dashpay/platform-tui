use std::borrow::Cow;

use dpp::{
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0,
    },
    prelude::{Identity, IdentityPublicKey},
};

use crate::ui::IdentityBalance;

pub struct TabbedString<'s> {
    pub indent: usize,
    pub content: Cow<'s, str>,
}

impl<'s> TabbedString<'s> {
    const SPACES_PER_INDENT: usize = 2;

    pub fn new(indent: usize, content: Cow<'s, str>) -> Self {
        TabbedString { indent, content }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{:indent$}{}",
            "",
            self.content,
            indent = self.indent * Self::SPACES_PER_INDENT
        )
    }

    pub fn adjust_parent_indent(mut self, parent_indent: usize) -> Self {
        self.indent += parent_indent;
        self
    }
}

pub trait InfoDisplay {
    fn display_info_lines(&self, parent_indent: usize) -> impl Iterator<Item = TabbedString>;
}

impl InfoDisplay for Identity {
    fn display_info_lines(&self, parent_indent: usize) -> impl Iterator<Item = TabbedString> {
        let identity_lines = [
            TabbedString::new(0, "Identity:".into()),
            TabbedString::new(1, format!("Id: {}", self.id()).into()),
            TabbedString::new(
                1,
                format!(
                    "Balance: {}",
                    IdentityBalance::from_credits(self.balance()).dash_str()
                )
                .into(),
            ),
            TabbedString::new(1, format!("Revision: {}", self.revision()).into()),
            TabbedString::new(1, "Public Keys:".into()),
        ]
        .into_iter();

        let public_keys_lines = self
            .public_keys()
            .values()
            .map(|pk| pk.display_info_lines(2))
            .flatten();

        identity_lines
            .chain(public_keys_lines)
            .map(move |s| s.adjust_parent_indent(parent_indent))
    }
}

impl InfoDisplay for IdentityPublicKey {
    fn display_info_lines(&self, parent_indent: usize) -> impl Iterator<Item = TabbedString> {
        [
            TabbedString::new(0, format!("{} key:", self.purpose()).into()),
            TabbedString::new(1, format!("Type: {}", self.key_type()).into()),
            TabbedString::new(1, format!("Security: {}", self.security_level()).into()),
        ]
        .into_iter()
        .map(move |s| s.adjust_parent_indent(parent_indent))
    }
}

pub(crate) fn display_info(value: &impl InfoDisplay) -> String {
    value
        .display_info_lines(0)
        .into_iter()
        .map(|tabbed_string| tabbed_string.to_string())
        .collect::<Vec<String>>()
        .join("\n")
}
