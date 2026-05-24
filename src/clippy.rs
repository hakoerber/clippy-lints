use std::fmt;

pub use crate::common::{Lint, LintGroup, LintId};

#[derive(Clone, Copy, Debug)]
pub enum LintLevel {
    Allow,
    Warn,
    Deny,
}

impl LintLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
        }
    }
}

impl fmt::Display for LintLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

include!(concat!(env!("OUT_DIR"), "/clippy_lints.rs"));

pub fn get_lints() -> &'static [Lint] {
    LINTS
}
