//! Generate documentation from Zener source files.
//!
//! This crate parses `.zen` files from a package directory, extracts docstrings
//! and module signatures, and generates markdown documentation.

// Use pipe-safe replacements for standard printing macros in CLI output paths.
#[macro_use(eprintln)]
extern crate anstream;

mod parser;
mod render;
mod signature;
mod types;

use anyhow::{Context, Result};
use pcb_zen_core::DefaultFileProvider;
use serde_json::Value as JsonValue;
use starlark::collections::SmallMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use types::*;

/// Generate documentation for a Zener package.
///
/// - `package_url`: Used as the h1 header (e.g. "github.com/diodeinc/stdlib")
/// - `display_path`: Path shown in source comment; defaults to package_root if None
/// - `filter`: Optional path prefix to filter files (e.g. "generics" or "Module.zen")
pub fn generate_docs(
    package_root: &Path,
    package_url: Option<&str>,
    display_path: Option<&str>,
    filter: Option<&str>,
) -> Result<DocsResult> {
    generate_docs_with_inputs(
        package_root,
        package_url,
        display_path,
        filter,
        &SmallMap::new(),
    )
}

/// Generate documentation for one evaluated root configuration. Overrides are
/// restricted to a single file, rather than applied to unrelated modules.
pub fn generate_docs_with_inputs(
    package_root: &Path,
    package_url: Option<&str>,
    display_path: Option<&str>,
    filter: Option<&str>,
    inputs: &SmallMap<String, JsonValue>,
) -> Result<DocsResult> {
    // Canonicalize to ensure consistent path handling
    let package_root = package_root
        .canonicalize()
        .unwrap_or_else(|_| package_root.to_path_buf());
    let zen_files = collect_zen_files(&package_root, filter)?;
    if !inputs.is_empty() && zen_files.len() != 1 {
        anyhow::bail!("--config documentation requires a single .zen file target");
    }
    let file_provider = DefaultFileProvider::new();
    let workspace_info =
        pcb_zen::get_workspace_info(&file_provider, &package_root).with_context(|| {
            format!(
                "Failed to load workspace info for {}",
                package_root.display()
            )
        })?;
    let resolution = pcb_zen::resolve_workspace_dependencies(workspace_info, &package_root, false)
        .with_context(|| {
            format!(
                "Failed to resolve dependencies for {}",
                package_root.display()
            )
        })?;

    let mut files = Vec::new();

    for path in zen_files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let file_path = get_file_path(&package_root, &path);

        match signature::try_get_signature(&path, &resolution, inputs) {
            signature::SignatureResult::Module(sig) => {
                for name in inputs.keys() {
                    if !sig.configs.iter().any(|param| &param.name == name) {
                        anyhow::bail!("Unknown config parameter '{name}' in {file_path}");
                    }
                }
                files.push(FileDoc::Module(ModuleDoc {
                    path: file_path,
                    file_doc: parser::extract_file_docstring(&content),
                    signature: sig,
                }));
            }
            signature::SignatureResult::Library => {
                if !inputs.is_empty() {
                    anyhow::bail!("{file_path} has no root config parameters");
                }
                files.push(FileDoc::Library(parser::parse_library(
                    file_path, &content,
                )?));
            }
            signature::SignatureResult::Error(e) => {
                if !inputs.is_empty() {
                    return Err(e).with_context(|| {
                        format!("Failed to evaluate requested configuration for {file_path}")
                    });
                }
                eprintln!("Warning: Failed to parse {}: {}", file_path, e);
            }
        }
    }

    // Sort by path
    files.sort_by(|a, b| a.path().cmp(b.path()));

    let default_path = package_root.to_string_lossy();
    let local_path = display_path.unwrap_or(&default_path);
    let mut markdown = render::render_docs(&files, package_url, Some(local_path));
    if files.iter().any(|file| matches!(file, FileDoc::Module(_))) {
        let scope = if inputs.is_empty() {
            "Module IO is evaluated with default config values. For a specific configuration, target one .zen file and repeat `--config KEY=VALUE`.".to_owned()
        } else {
            let overrides = inputs
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Module IO is evaluated with these root config overrides: {}",
                overrides.replace('`', "\\`").replace('\n', " ")
            )
        };
        markdown = format!("{scope}\n\n{markdown}");
    }

    let (library_count, module_count) = files.iter().fold((0, 0), |(l, m), f| match f {
        FileDoc::Library(_) => (l + 1, m),
        FileDoc::Module(_) => (l, m + 1),
    });

    Ok(DocsResult {
        markdown,
        library_count,
        module_count,
    })
}

/// Collect all .zen files, excluding test/ and hidden directories.
///
/// If `filter` is provided, only files whose relative path starts with the filter
/// prefix are included. The filter can be a directory prefix (e.g., "generics")
/// or a specific file (e.g., "Module.zen").
fn collect_zen_files(root: &Path, filter: Option<&str>) -> Result<Vec<PathBuf>> {
    // Canonicalize root to ensure strip_prefix works correctly with WalkDir paths
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let mut files: Vec<_> = WalkDir::new(&canonical_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "zen"))
        .filter(|e| {
            // Only check path components relative to root, not the absolute path
            let rel_path = e.path().strip_prefix(&canonical_root).unwrap_or(e.path());
            !rel_path.components().any(|c| {
                let s = c.as_os_str().to_string_lossy();
                s == "test" || s.starts_with('.')
            })
        })
        .filter(|e| {
            // Apply filter if provided
            if let Some(filter) = filter {
                let rel_path = e.path().strip_prefix(&canonical_root).unwrap_or(e.path());
                let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                // Match if relative path starts with filter or equals filter
                rel_str.starts_with(filter) || rel_str == filter
            } else {
                true
            }
        })
        .map(|e| e.into_path())
        .collect();

    files.sort();
    Ok(files)
}

/// Get the file path relative to package root.
fn get_file_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Result of the documentation generation.
pub struct DocsResult {
    pub markdown: String,
    pub library_count: usize,
    pub module_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("pcb.toml"), "[dependencies]\n").unwrap();
        fs::write(
            directory.path().join("hole.zen"),
            r#"
Net = builtin.net_type("Net")
Mode = enum("plated", "mechanical")
mode = config(Mode, default=Mode("plated"))
P1 = builtin.io(Net, optional=True) if mode != Mode("mechanical") else None
"#,
        )
        .unwrap();
        directory
    }

    #[test]
    fn io_docs_follow_the_evaluated_configuration() {
        let directory = fixture();
        let defaults = generate_docs(directory.path(), None, None, Some("hole.zen")).unwrap();
        assert!(defaults.markdown.contains("| P1 | Net |  | yes |"));
        assert!(defaults.markdown.contains("default config values"));
        let inputs = SmallMap::from_iter([(
            "mode".to_owned(),
            JsonValue::String("mechanical".to_owned()),
        )]);
        let configured =
            generate_docs_with_inputs(directory.path(), None, None, Some("hole.zen"), &inputs)
                .unwrap();
        assert!(!configured.markdown.contains("| P1 |"));
        assert!(
            configured
                .markdown
                .contains("**IO:** none in this configuration.")
        );
        assert!(configured.markdown.contains("mode=\"mechanical\""));
    }

    #[test]
    fn configured_docs_reject_invalid_values_unknown_keys_and_multiple_files() {
        let directory = fixture();
        for (key, value) in [("mode", "invalid"), ("typo", "mechanical")] {
            let inputs =
                SmallMap::from_iter([(key.to_owned(), JsonValue::String(value.to_owned()))]);
            assert!(
                generate_docs_with_inputs(directory.path(), None, None, Some("hole.zen"), &inputs)
                    .is_err()
            );
        }
        fs::write(
            directory.path().join("other.zen"),
            "count = config(int, default=1)\n",
        )
        .unwrap();
        let inputs = SmallMap::from_iter([(
            "mode".to_owned(),
            JsonValue::String("mechanical".to_owned()),
        )]);
        let error = generate_docs_with_inputs(directory.path(), None, None, None, &inputs)
            .err()
            .unwrap();
        assert!(error.to_string().contains("single .zen file"));
    }
}
