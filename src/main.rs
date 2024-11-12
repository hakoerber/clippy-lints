use std::fmt::{self, Write as _};

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum LintGroup {
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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Profile {
    Publish,
    Personal,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    profile: Profile,

    #[arg(long)]
    workspace: bool,
}

impl LintGroup {
    fn as_str(self) -> &'static str {
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

impl fmt::Display for LintGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LintLevel {
    Allow,
    Warn,
    Deny,
    None,
}

impl LintLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::None => "none",
        }
    }
}

#[derive(Debug)]
struct Lint<'a> {
    id: LintId<'a>,
    group: LintGroup,
}

#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "this is an external data definition")]
struct LintResponse {
    id: String,
    group: LintGroup,
    #[serde(rename = "level")]
    default_level: LintLevel,
    version: String,
}

#[derive(Clone, Copy, Debug)]
enum PrioritySetting {
    Explicit(isize),
    Unspecified,
}

impl From<Option<isize>> for PrioritySetting {
    fn from(value: Option<isize>) -> Self {
        match value {
            Some(i) => Self::Explicit(i),
            None => Self::Unspecified,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct LintId<'a>(&'a str);

impl From<&'static str> for LintId<'static> {
    fn from(value: &'static str) -> Self {
        Self(value)
    }
}

impl fmt::Display for LintId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct LintList<'a>(Vec<LintId<'a>>);

impl<'a> From<Vec<&'a str>> for LintList<'a> {
    fn from(value: Vec<&'a str>) -> Self {
        Self(value.into_iter().map(|s| LintId(s)).collect())
    }
}

#[derive(Debug)]
struct SingleLintConfig<'a> {
    lint: &'a LintId<'a>,
    priority: PrioritySetting,
    level: LintLevel,
}

#[derive(Debug)]
struct GroupConfig {
    group: LintGroup,
    priority: PrioritySetting,
    level: LintLevel,
}

#[derive(Debug)]
enum Setting<'a> {
    Single(SingleLintConfig<'a>),
    Group(GroupConfig),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExhaustiveGroupClassification {
    Default,
    Exception,
}

#[derive(Debug)]
struct ExhausiveGroup<'a> {
    defaults: Vec<Setting<'a>>,
    exceptions: Vec<Setting<'a>>,
}

struct Exceptions<'a> {
    level: LintLevel,
    lints: LintList<'a>,
}

impl<'a> Setting<'a> {
    fn group(group: LintGroup, level: LintLevel, priority: impl Into<PrioritySetting>) -> Self {
        Self::Group(GroupConfig {
            group,
            priority: priority.into(),
            level,
        })
    }

    fn allow(
        all_lints: &'a AllLints,
        group: LintGroup,
        lints: &'a [LintId<'a>],
    ) -> Result<Vec<Self>> {
        lints
            .iter()
            .map(|lint| {
                let found = all_lints
                    .0
                    .iter()
                    .find(|r| r.id == *lint && r.group == group);
                if found.is_none() {
                    Err(anyhow!("lint {} not in group {}", lint, group.as_str()))
                } else {
                    Ok(Self::Single(SingleLintConfig {
                        lint,
                        priority: PrioritySetting::Unspecified,
                        level: LintLevel::Allow,
                    }))
                }
            })
            .collect()
    }

    fn split_group_exhaustive(
        all_lints: &'a AllLints,
        group: LintGroup,
        default_level: LintLevel,
        exceptions: &Exceptions<'a>,
    ) -> Result<ExhausiveGroup<'a>> {
        let all_lints_in_group: Vec<&LintId> = all_lints
            .0
            .iter()
            .filter(|lint| lint.group == group)
            .map(|lint| &lint.id)
            .collect();

        let all_lints_in_group_len = all_lints_in_group.len();

        exceptions
            .lints
            .0
            .iter()
            .find(|lint| (!all_lints_in_group.contains(lint)))
            .map(|lint| Err(anyhow!("lint {lint} not part of group {group}")))
            .unwrap_or(Ok(()))?;

        Ok(all_lints_in_group
            .into_iter()
            .map(|lint| {
                if exceptions.lints.0.contains(lint) {
                    (
                        ExhaustiveGroupClassification::Exception,
                        Self::Single(SingleLintConfig {
                            lint,
                            priority: PrioritySetting::Unspecified,
                            level: exceptions.level,
                        }),
                    )
                } else {
                    (
                        ExhaustiveGroupClassification::Default,
                        Self::Single(SingleLintConfig {
                            lint,
                            priority: PrioritySetting::Unspecified,
                            level: default_level,
                        }),
                    )
                }
            })
            .fold(
                {
                    let len_1 = exceptions.lints.0.len();
                    ExhausiveGroup {
                        defaults: Vec::with_capacity(
                            all_lints_in_group_len.checked_sub(len_1).expect(
                                "exceptions are a subset of of all lints in group, checked above",
                            ),
                        ),
                        exceptions: Vec::with_capacity(len_1),
                    }
                },
                |mut acc, (classification, setting)| {
                    match classification {
                        ExhaustiveGroupClassification::Default => acc.defaults.push(setting),
                        ExhaustiveGroupClassification::Exception => acc.exceptions.push(setting),
                    };
                    acc
                },
            ))
    }
}

#[derive(Debug)]
struct ConfigGroup<'a> {
    comment: Option<String>,
    settings: Vec<Setting<'a>>,
}

#[derive(Debug)]
struct Config<'a>(Vec<ConfigGroup<'a>>);

impl Config<'_> {
    fn to_toml(&self, args: &Args) -> String {
        let mut output = if args.workspace {
            String::from("[workspace.lints.clippy]\n")
        } else {
            String::from("[lints.clippy]\n")
        };

        let mut iter_group = self.0.iter().peekable();

        while let Some(group) = iter_group.next() {
            let last_group = iter_group.peek().is_none();
            if let Some(ref comment) = group.comment {
                writeln!(output, "# {comment}").expect("writing to string succeeds");
            }

            let mut iter_setting = group.settings.iter().peekable();
            while let Some(setting) = iter_setting.next() {
                let last_setting = iter_setting.peek().is_none();
                match *setting {
                    Setting::Single(ref single_lint_config) => match single_lint_config.priority {
                        PrioritySetting::Explicit(priority) => write!(
                            output,
                            "{} = {{ level = \"{}\", priority = {} }}",
                            single_lint_config.lint.0,
                            single_lint_config.level.as_str(),
                            priority
                        )
                        .expect("writing to string succeeds"),
                        PrioritySetting::Unspecified => write!(
                            output,
                            "{} = \"{}\"",
                            single_lint_config.lint.0,
                            single_lint_config.level.as_str()
                        )
                        .expect("writing to string succeeds"),
                    },
                    Setting::Group(ref group_config) => match group_config.priority {
                        PrioritySetting::Explicit(priority) => write!(
                            output,
                            "{} = {{ level = \"{}\", priority = {} }}",
                            group_config.group.as_str(),
                            group_config.level.as_str(),
                            priority
                        )
                        .expect("writing to string succeeds"),
                        PrioritySetting::Unspecified => write!(
                            output,
                            "{} = \"{}\"",
                            group_config.group.as_str(),
                            group_config.level.as_str(),
                        )
                        .expect("writing to string succeeds"),
                    },
                };
                if !last_setting {
                    output.push('\n');
                }
                if last_setting && !last_group {
                    output.push('\n');
                }
            }
            if !last_group {
                output.push('\n');
            }
        }

        output
    }
}

#[derive(Debug, Deserialize)]
struct Response(Vec<LintResponse>);

#[derive(Debug)]
struct AllLints<'a>(Vec<Lint<'a>>);

impl<'a> AllLints<'a> {
    fn from_response(response: &'a Response) -> Self {
        Self(
            response
                .0
                .iter()
                .map(|lint| Lint {
                    id: LintId(&lint.id),
                    group: lint.group,
                })
                .collect(),
        )
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let response: Response = ureq::get("https://rust-lang.github.io/rust-clippy/stable/lints.json")
        .call()?
        .into_json::<Response>()?;

    let all_lints = AllLints::from_response(&response);

    let restriction_group = Setting::split_group_exhaustive(
        &all_lints,
        LintGroup::Restriction,
        LintLevel::Allow,
        &Exceptions {
            level: LintLevel::Warn,
            lints: vec![
                "allow_attributes",
                "allow_attributes_without_reason",
                "arithmetic_side_effects",
                "as_conversions",
                "assertions_on_result_states",
                "cfg_not_test",
                "clone_on_ref_ptr",
                "create_dir",
                "dbg_macro",
                "decimal_literal_representation",
                "default_numeric_fallback",
                "deref_by_slicing",
                "disallowed_script_idents",
                "else_if_without_else",
                "empty_drop",
                "empty_enum_variants_with_brackets",
                "empty_structs_with_brackets",
                "exit",
                "filetype_is_file",
                "float_arithmetic",
                "float_cmp_const",
                "fn_to_numeric_cast_any",
                "format_push_string",
                "get_unwrap",
                "indexing_slicing",
                "infinite_loop",
                "inline_asm_x86_att_syntax",
                "inline_asm_x86_intel_syntax",
                "integer_division",
                "iter_over_hash_type",
                "large_include_file",
                "let_underscore_must_use",
                "let_underscore_untyped",
                "little_endian_bytes",
                "lossy_float_literal",
                "map_err_ignore",
                "mem_forget",
                "missing_assert_message",
                "missing_asserts_for_indexing",
                "mixed_read_write_in_expression",
                "modulo_arithmetic",
                "multiple_inherent_impl",
                "multiple_unsafe_ops_per_block",
                "mutex_atomic",
                "panic",
                "partial_pub_fields",
                "pattern_type_mismatch",
                "print_stderr",
                "print_stdout",
                "pub_without_shorthand",
                "rc_buffer",
                "rc_mutex",
                "redundant_type_annotations",
                "renamed_function_params",
                "rest_pat_in_fully_bound_structs",
                "same_name_method",
                "self_named_module_files",
                "semicolon_inside_block",
                "str_to_string",
                "string_add",
                "string_lit_chars_any",
                "string_slice",
                "string_to_string",
                "suspicious_xor_used_as_pow",
                "tests_outside_test_module",
                "todo",
                "try_err",
                "undocumented_unsafe_blocks",
                "unimplemented",
                "unnecessary_safety_comment",
                "unnecessary_safety_doc",
                "unnecessary_self_imports",
                "unneeded_field_pattern",
                "unseparated_literal_suffix",
                "unused_result_ok",
                "unwrap_used",
                "use_debug",
                "verbose_file_reads",
            ]
            .into(),
        },
    )?;

    let cargo_lints = {
        let mut v = vec!["multiple_crate_versions".into()];
        match args.profile {
            Profile::Publish => (),
            Profile::Personal => v.push("cargo_common_metadata".into()),
        }
        v
    };

    let pedantic_allows = &[
        "too_many_lines".into(),
        "must_use_candidate".into(),
        "map_unwrap_or".into(),
        "missing_errors_doc".into(),
        "if_not_else".into(),
    ];

    let nursery_allows = &[
        "missing_const_for_fn".into(),
        "option_if_let_else".into(),
        "redundant_pub_crate".into(),
    ];

    let complexity_allows = &["too_many_arguments".into()];

    let style_allows = &["new_without_default".into(), "redundant_closure".into()];

    let config = Config(vec![
        ConfigGroup {
            comment: Some("enabled groups".to_owned()),
            settings: vec![
                Setting::group(LintGroup::Correctness, LintLevel::Deny, Some(-1)),
                Setting::group(LintGroup::Suspicious, LintLevel::Warn, Some(-1)),
                Setting::group(LintGroup::Style, LintLevel::Warn, Some(-1)),
                Setting::group(LintGroup::Complexity, LintLevel::Warn, Some(-1)),
                Setting::group(LintGroup::Perf, LintLevel::Warn, Some(-1)),
                Setting::group(LintGroup::Cargo, LintLevel::Warn, Some(-1)),
                Setting::group(LintGroup::Pedantic, LintLevel::Warn, Some(-1)),
                Setting::group(LintGroup::Nursery, LintLevel::Warn, Some(-1)),
            ],
        },
        ConfigGroup {
            comment: Some("pedantic overrides".to_owned()),
            settings: Setting::allow(&all_lints, LintGroup::Pedantic, pedantic_allows)?,
        },
        ConfigGroup {
            comment: Some("nursery overrides".to_owned()),
            settings: Setting::allow(&all_lints, LintGroup::Nursery, nursery_allows)?,
        },
        ConfigGroup {
            comment: Some("complexity overrides".to_owned()),
            settings: Setting::allow(&all_lints, LintGroup::Complexity, complexity_allows)?,
        },
        ConfigGroup {
            comment: Some("style overrides".to_owned()),
            settings: Setting::allow(&all_lints, LintGroup::Style, style_allows)?,
        },
        ConfigGroup {
            comment: Some("cargo overrides".to_owned()),
            settings: Setting::allow(&all_lints, LintGroup::Cargo, &cargo_lints)?,
        },
        ConfigGroup {
            comment: Some("selected restrictions".to_owned()),
            settings: restriction_group.exceptions,
        },
        ConfigGroup {
            comment: Some("restrictions explicit allows".to_owned()),
            settings: restriction_group.defaults,
        },
    ]);

    let output = config.to_toml(&args);

    #[expect(clippy::print_stdout, reason = "this is the main program output")]
    {
        println!("{output}");
    }

    Ok(())
}
