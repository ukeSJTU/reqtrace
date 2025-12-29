mod file_discovery;
mod requirement;
mod scanner;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use file_discovery::{discover_code_files, discover_requirement_files};
use requirement::RequirementStore;
use scanner::{CodeScanner, TraceabilityReport};
use std::collections::HashSet;

#[derive(Parser)]
#[command(name = "reqtrace")]
#[command(about = "Modern code requirement traceability engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan and analyze traceability between requirements and code
    Check {
        /// Path to requirement file, directory, or glob pattern (e.g., "requirements/", "*.md", "reqs/**/*.md")
        #[arg(short, long)]
        requirement: String,

        /// Path(s) to source code files, directories, or glob patterns (e.g., "src/", "tests/**/*.py")
        #[arg(short, long)]
        code: Vec<String>,
    },

    /// Generate a traceability report
    Report {
        /// Path to requirement file, directory, or glob pattern
        #[arg(short, long)]
        requirement: String,

        /// Path(s) to source code files, directories, or glob patterns
        #[arg(short, long)]
        code: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { requirement, code } => {
            check_traceability(requirement, code)?;
        }
        Commands::Report { requirement, code } => {
            generate_report(requirement, code)?;
        }
    }

    Ok(())
}

fn check_traceability(req_input: String, code_inputs: Vec<String>) -> Result<()> {
    // Discover requirement files
    println!("Discovering requirement files from: {}", req_input);
    let (req_files, req_disc_errors) = discover_requirement_files(&req_input)?;

    if !req_disc_errors.is_empty() {
        eprintln!("\nErrors discovering requirement files:");
        for (path, err) in &req_disc_errors {
            eprintln!("  ✗ {}: {}", path.display(), err);
        }
    }

    if req_files.is_empty() {
        bail!("Error: No requirement files found for: {}", req_input);
    }

    println!("Found {} requirement file(s)", req_files.len());

    // Load all requirements
    let (req_store, req_errors) = RequirementStore::from_paths(req_files);

    // Report requirement loading errors
    if !req_errors.is_empty() {
        eprintln!("\nErrors loading requirements:");
        for (path, err) in &req_errors {
            eprintln!("  ✗ {}: {}", path.display(), err);
        }
    }

    if req_store.is_empty() {
        bail!("Error: No valid requirements loaded");
    }

    println!("Loaded {} requirement(s) successfully", req_store.len());
    for req in req_store.all() {
        println!("  - {}: {}", req.meta.id, req.meta.title);
    }

    // Discover code files
    println!("\nDiscovering code files...");
    let (code_files, code_disc_errors) = discover_code_files(&code_inputs)?;

    if !code_disc_errors.is_empty() {
        eprintln!("\nErrors discovering code files:");
        for (path, err) in &code_disc_errors {
            eprintln!("  ✗ {}: {}", path.display(), err);
        }
    }

    if code_files.is_empty() {
        bail!("Error: No code files found");
    }

    println!("Found {} code file(s)", code_files.len());

    // Scan code files in parallel
    println!("\nScanning code files in parallel...");
    let scanner = CodeScanner::new()?;
    let (all_references, scan_errors) = scanner.scan_files_parallel(code_files);

    // Report scanning errors
    if !scan_errors.is_empty() {
        eprintln!("\nErrors scanning code files:");
        for (path, err) in &scan_errors {
            eprintln!("  ✗ {}: {}", path.display(), err);
        }
    }

    println!("Found {} trace reference(s)", all_references.len());

    // Analyze coverage
    let all_req_ids: HashSet<String> = req_store.get_all_ids().into_iter().collect();
    let covered_ids: HashSet<String> = all_references.iter().map(|r| r.req_id.clone()).collect();

    let uncovered_ids: Vec<String> = all_req_ids.difference(&covered_ids).cloned().collect();

    // Create report
    let report = TraceabilityReport {
        total_requirements: all_req_ids.len(),
        covered_requirements: covered_ids.len(),
        references: all_references,
        uncovered_ids,
    };

    report.print_summary();

    // Exit with error if there were any errors or coverage is incomplete
    let has_errors = !req_errors.is_empty()
        || !scan_errors.is_empty()
        || !req_disc_errors.is_empty()
        || !code_disc_errors.is_empty();
    let incomplete_coverage = report.covered_requirements < report.total_requirements;

    if has_errors || incomplete_coverage {
        if incomplete_coverage {
            println!("Not all requirements are traced!");
        }
        if has_errors {
            println!("Some files failed to process!");
        }
        bail!("Traceability check failed");
    } else {
        println!("All requirements are properly traced!");
    }

    Ok(())
}

fn generate_report(req_input: String, code_inputs: Vec<String>) -> Result<()> {
    // For now, just run the same check - in future this will generate HTML
    check_traceability(req_input, code_inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_check_traceability_no_requirements() {
        // Use a glob that matches nothing
        let res = check_traceability("nonexistent-*.md".to_string(), vec!["src/*.rs".to_string()]);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.to_string().contains("No requirement files found"));
    }

    #[test]
    fn test_check_traceability_no_code_files() {
        // Create a valid requirement file in a temp dir
        let dir = tempdir().expect("tempdir");
        let req_path = dir.path().join("req.md");
        let mut f = File::create(&req_path).expect("create file");
        let content = r#"---
id: "REQ-TEST"
title: "Test Requirement"
---

### AC-1
This is an acceptance criterion.
"#;
        f.write_all(content.as_bytes()).expect("write");

        // Use a code glob that matches nothing
        let res = check_traceability(
            req_path.to_string_lossy().to_string(),
            vec!["nonexistent-*.rs".to_string()],
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.to_string().contains("No code files found"));
    }

    #[test]
    fn test_check_traceability_invalid_requirement_file() {
        // Create an invalid requirement file (no frontmatter)
        let dir = tempdir().expect("tempdir");
        let req_path = dir.path().join("bad_req.md");
        let mut f = File::create(&req_path).expect("create file");
        f.write_all(b"This file has no frontmatter").expect("write");

        // Use an existing code glob (repo src/*.rs should exist)
        let res = check_traceability(
            req_path.to_string_lossy().to_string(),
            vec!["src/*.rs".to_string()],
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.to_string().contains("No valid requirements loaded"));
    }
}
