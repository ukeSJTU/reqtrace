use anyhow::{Context, Result};
use globset::Glob;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::discovery::DEFAULT_EXCLUDED_DIRS;

/// Discovers requirement files from a given path.
///
/// Input can be:
/// - A single file: returns that file if it exists
/// - A directory: recursively scans for `**/*.md` files
/// - A glob pattern: matches files against the pattern (relative to current working directory)
/// Discover requirement files and return (files, discovery_errors)
pub fn discover_requirement_files(
    path: &str,
) -> Result<(Vec<PathBuf>, Vec<(PathBuf, anyhow::Error)>)> {
    discover_files_from_path(path, Some("md"))
}

/// Discovers code files from a list of paths.
///
/// Each input can be:
/// - A single file: includes that file
/// - A directory: recursively scans all files (TODO: add exclude patterns)
/// - A glob pattern: matches files against the pattern (relative to current working directory)
///
/// All discovered files are collected and deduplicated.
/// Discover code files from a list of paths and return (files, discovery_errors)
pub fn discover_code_files(
    paths: &[String],
) -> Result<(Vec<PathBuf>, Vec<(PathBuf, anyhow::Error)>)> {
    let mut all_files = Vec::new();
    let mut all_errors: Vec<(PathBuf, anyhow::Error)> = Vec::new();

    for path_str in paths {
        let (mut files, mut errs) = discover_files_from_path(path_str, None)?;
        all_files.append(&mut files);
        all_errors.append(&mut errs);
    }

    // Deduplicate files
    all_files.sort();
    all_files.dedup();

    Ok((all_files, all_errors))
}

// Helper that resolves a path-like string into a list of files. The path may be:
// - a direct file path
// - a directory, in which case files are recursively scanned; if `dir_extension_filter`
//   is Some(ext) only files with that extension will be included
// - a glob pattern, in which case the glob is matched against files under a
//   reasonable base directory derived from the pattern
fn discover_files_from_path(
    path_str: &str,
    dir_extension_filter: Option<&str>,
) -> Result<(Vec<PathBuf>, Vec<(PathBuf, anyhow::Error)>)> {
    let mut discovery_errors: Vec<(PathBuf, anyhow::Error)> = Vec::new();
    let path_obj = Path::new(path_str);

    // Case 1: Direct file
    if path_obj.is_file() {
        return Ok((vec![path_obj.to_path_buf()], discovery_errors));
    }

    // Case 2: Directory
    if path_obj.is_dir() {
        let mut files = Vec::new();
        for entry_result in WalkDir::new(path_obj)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip excluded directories
                if e.file_type().is_dir() {
                    let dir_name = e.file_name().to_string_lossy();
                    !DEFAULT_EXCLUDED_DIRS.contains(&dir_name.as_ref())
                } else {
                    true
                }
            })
        {
            match entry_result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = dir_extension_filter {
                            if path.extension().and_then(|s| s.to_str()) == Some(ext) {
                                files.push(path.to_path_buf());
                            }
                        } else {
                            files.push(path.to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    let p = e
                        .path()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| path_obj.to_path_buf());
                    discovery_errors.push((p, e.into()));
                }
            }
        }
        return Ok((files, discovery_errors));
    }

    // Case 3: Glob pattern
    let glob = Glob::new(path_str)
        .with_context(|| format!("Invalid glob pattern: {}", path_str))?
        .compile_matcher();

    let mut files = Vec::new();

    // Derive base directory to limit the WalkDir search. Use Path and its
    // components to be cross-platform (don't assume '/' as the separator).
    let mut base_dir = PathBuf::new();
    for component in Path::new(path_str).components() {
        let part_str = component.as_os_str().to_string_lossy();
        if part_str.contains('*') || part_str.contains('?') || part_str.contains('[') {
            break;
        }
        base_dir.push(component.as_os_str());
    }

    // If no non-glob prefix was found (e.g. "*.rs"), search from current dir.
    if base_dir.as_os_str().is_empty() {
        base_dir.push(".");
    }

    for entry_result in WalkDir::new(&base_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip excluded directories
            if e.file_type().is_dir() {
                let dir_name = e.file_name().to_string_lossy();
                !DEFAULT_EXCLUDED_DIRS.contains(&dir_name.as_ref())
            } else {
                true
            }
        })
    {
        match entry_result {
            Ok(entry) => {
                let entry_path = entry.path();
                if entry_path.is_file() && glob.is_match(entry_path) {
                    if let Some(ext) = dir_extension_filter {
                        if entry_path.extension().and_then(|s| s.to_str()) == Some(ext) {
                            files.push(entry_path.to_path_buf());
                        }
                    } else {
                        files.push(entry_path.to_path_buf());
                    }
                }
            }
            Err(e) => {
                let p = e
                    .path()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(base_dir.clone()));
                discovery_errors.push((p, e.into()));
            }
        }
    }

    Ok((files, discovery_errors))
}

/// Checks if a path looks like a glob pattern (contains *, ?, [, or **)
#[allow(dead_code)]
pub fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_glob_pattern() {
        assert!(is_glob_pattern("*.md"));
        assert!(is_glob_pattern("tests/**/*.py"));
        assert!(is_glob_pattern("src/[abc].rs"));
        assert!(!is_glob_pattern("src/main.rs"));
        assert!(!is_glob_pattern("requirements/"));
    }
}
