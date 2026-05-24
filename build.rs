use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

#[path = "src/common.rs"]
mod common;

use crate::common::{Lint, LintGroup, LintId};

use proc_macro2::TokenTree;
use walkdir::WalkDir;

pub fn build_file_with_all_lints(root: &Path) -> Result<Vec<Lint>> {
    let mut out = Vec::new();

    let src_root = root.join("clippy_lints/src");

    for entry in WalkDir::new(src_root) {
        let entry = entry?;

        #[expect(clippy::filetype_is_file, reason = "dont want to parse symlinks")]
        if !entry.file_type().is_file() {
            continue;
        }

        if entry.path().extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let src = fs::read_to_string(entry.path())?;
        let Ok(file) = syn::parse_file(&src) else {
            continue;
        };

        for item in file.items {
            let syn::Item::Macro(mac) = item else {
                continue;
            };

            if !mac.mac.path.is_ident("declare_clippy_lint") {
                continue;
            }

            let mut tokens = mac.mac.tokens.into_iter();

            while let Some(tok) = tokens.next() {
                if let TokenTree::Ident(ident) = tok {
                    if ident == "pub" {
                        // lint name
                        let lint_id = match tokens.next() {
                            Some(TokenTree::Ident(id)) => id.to_string(),
                            _ => break,
                        };

                        // skip optional comma
                        if matches!(
                            tokens.clone().next(),
                            Some(TokenTree::Punct(p)) if p.as_char() == ','
                        ) {
                            tokens.next();
                        }

                        // group
                        let group = match tokens.next() {
                            Some(TokenTree::Ident(g)) => g.to_string(),
                            _ => break,
                        };

                        out.push(Lint {
                            id: LintId::new(lint_id.to_lowercase()),
                            group: group.parse()?,
                        });

                        break;
                    }
                }
            }
        }
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);

    Ok(out)
}
fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let clippy_submodule_path = manifest_dir.join("rust-clippy");
    let lints = build_file_with_all_lints(&clippy_submodule_path)?;

    let mut code = String::new();

    code.push_str("pub static LINTS: &[Lint] = &[\n");

    for lint in lints {
        write!(
            code,
            r#"
                Lint {{
                    id: LintId::new_static("{}"),
                    group: LintGroup::{},
                }},
            "#,
            lint.id.0,
            <LintGroup as std::convert::Into<&'static str>>::into(lint.group),
        )?;
    }

    code.push_str("];\n");

    fs::write(out_dir.join("clippy_lints.rs"), code)?;

    Ok(())
}
