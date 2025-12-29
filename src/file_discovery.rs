use anyhow::{Context, Result};
use globset::Glob;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discovers requirement files from a given path.
///
/// Input can be:
/// - A single file: returns that file if it exists
/// - A directory: recursively scans for `**/*.md` files
/// - A glob pattern: matches files against the pattern (relative to current working directory)
pub fn discover_requirement_files(path: &str) -> Result<Vec<PathBuf>> {
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
pub fn discover_code_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    for path_str in paths {
        let mut files = discover_files_from_path(path_str, None)?;
        all_files.append(&mut files);
    }

    // Deduplicate files
    all_files.sort();
    all_files.dedup();

    Ok(all_files)
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
) -> Result<Vec<PathBuf>> {
    let path_obj = Path::new(path_str);

    // Case 1: Direct file
    if path_obj.is_file() {
        return Ok(vec![path_obj.to_path_buf()]);
    }

    // Case 2: Directory
    if path_obj.is_dir() {
        let mut files = Vec::new();
        for entry in WalkDir::new(path_obj)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
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
        return Ok(files);
    }

    // Case 3: Glob pattern
    let glob = Glob::new(path_str)
        .with_context(|| format!("Invalid glob pattern: {}", path_str))?
        .compile_matcher();

    let mut files = Vec::new();

    // Derive base directory to limit the WalkDir search
    let base_dir = if path_str.contains('/') {
        let parts: Vec<&str> = path_str.split('/').collect();
        let mut base_parts = Vec::new();
        for part in parts {
            if part.contains('*') || part.contains('?') || part.contains('[') {
                break;
            }
            base_parts.push(part);
        }
        if base_parts.is_empty() {
            ".".to_string()
        } else {
            base_parts.join("/")
        }
    } else {
        ".".to_string()
    };

    for entry in WalkDir::new(base_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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

    Ok(files)
}

/// Checks if a path looks like a glob pattern (contains *, ?, [, or **)
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
