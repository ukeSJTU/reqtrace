mod requirement;
mod scanner;

use anyhow::Result;
use clap::{Parser, Subcommand};
use requirement::Requirement;
use scanner::{CodeScanner, TraceabilityReport};
use std::collections::HashSet;
use std::path::PathBuf;

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
        /// Path to requirement markdown file
        #[arg(short, long)]
        requirement: PathBuf,

        /// Path to source code file(s) to scan
        #[arg(short, long)]
        code: Vec<PathBuf>,
    },
    
    /// Generate a traceability report
    Report {
        /// Path to requirement markdown file
        #[arg(short, long)]
        requirement: PathBuf,

        /// Path to source code file(s) to scan
        #[arg(short, long)]
        code: Vec<PathBuf>,
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

fn check_traceability(req_path: PathBuf, code_paths: Vec<PathBuf>) -> Result<()> {
    // Parse requirement
    println!("Parsing requirement: {}", req_path.display());
    let req = Requirement::from_file(&req_path)?;
    
    println!("   ID: {}", req.meta.id);
    println!("   Title: {}", req.meta.title);
    println!("   Hash: {}", &req.hash[..16]);
    
    if !req.acceptance_criteria.is_empty() {
        println!("   Acceptance Criteria: {}", req.acceptance_criteria.len());
        for ac in &req.acceptance_criteria {
            println!("     - {}: {}", ac.id, ac.title);
        }
    }

    // Scan code files
    println!("\nScanning code files...");
    let mut scanner = CodeScanner::new()?;
    let mut all_references = Vec::new();

    for code_path in &code_paths {
        println!("   Scanning: {}", code_path.display());
        let references = scanner.scan_file(code_path)?;
        println!("     Found {} references", references.len());
        all_references.extend(references);
    }

    // Analyze coverage
    let all_req_ids: HashSet<String> = req.get_all_ids().into_iter().collect();
    let covered_ids: HashSet<String> = all_references
        .iter()
        .map(|r| r.req_id.clone())
        .collect();

    let uncovered_ids: Vec<String> = all_req_ids
        .difference(&covered_ids)
        .cloned()
        .collect();

    // Create report
    let report = TraceabilityReport {
        total_requirements: all_req_ids.len(),
        covered_requirements: covered_ids.len(),
        references: all_references,
        uncovered_ids,
    };

    report.print_summary();

    // Exit with error if coverage is incomplete
    if report.covered_requirements < report.total_requirements {
        println!("Not all requirements are traced!");
        std::process::exit(1);
    } else {
        println!("All requirements are properly traced!");
    }

    Ok(())
}

fn generate_report(req_path: PathBuf, code_paths: Vec<PathBuf>) -> Result<()> {
    // For now, just run the same check - in future this will generate HTML
    check_traceability(req_path, code_paths)
}
