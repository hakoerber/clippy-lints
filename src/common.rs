use std::{borrow::Cow, fmt};

use anyhow::bail;

#[derive(Debug)]
pub struct Lint {
    pub id: LintId,
    pub group: LintGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum_macros::IntoStaticStr)]
pub enum LintGroup {
    Cargo,
    Complexity,
    Correctness,
    Nursery,
    Pedantic,
    Perf,
    Restriction,
    Style,
    Suspicious,
    Deprecated,
}

impl LintGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Complexity => "complexity",
            Self::Correctness => "correctness",
            Self::Nursery => "nursery",
            Self::Pedantic => "pedantic",
            Self::Perf => "perf",
            Self::Restriction => "restriction",
            Self::Style => "style",
            Self::Suspicious => "suspicious",
            Self::Deprecated => "deprecated",
        }
    }
}

impl std::str::FromStr for LintGroup {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "cargo" => Self::Cargo,
            "complexity" => Self::Complexity,
            "correctness" => Self::Correctness,
            "nursery" => Self::Nursery,
            "pedantic" => Self::Pedantic,
            "perf" => Self::Perf,
            "restriction" => Self::Restriction,
            "style" => Self::Style,
            "suspicious" => Self::Suspicious,
            "deprecated" => Self::Deprecated,
            _ => bail!("unknown lint group"),
        })
    }
}

impl fmt::Display for LintGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LintId(pub Cow<'static, str>);

impl LintId {
    pub fn new(val: String) -> Self {
        Self(Cow::Owned(val))
    }

    pub const fn new_static(val: &'static str) -> Self {
        Self(Cow::Borrowed(val))
    }
}

impl From<&'static str> for LintId {
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl fmt::Display for LintId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
