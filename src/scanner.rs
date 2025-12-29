use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

use crate::config::scanner::*;

/// Cached regex for extracting @reqtrace:ID patterns
static REQTRACE_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(REQTRACE_PATTERN).expect("Invalid reqtrace regex pattern"));

/// Reference to a requirement found in code
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceReference {
    pub req_id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
}

/// Scanner for finding @reqtrace comments in code
#[derive(Clone)]
pub struct CodeScanner {
    // Store language info, parsers will be created per-thread
    languages: HashMap<String, Language>,
}

impl CodeScanner {
    pub fn new() -> Result<Self> {
        let mut scanner = Self {
            languages: HashMap::new(),
        };

        // Initialize language support
        scanner.init_python()?;
        // Add more languages as needed

        Ok(scanner)
    }

    fn init_python(&mut self) -> Result<()> {
        let language = tree_sitter_python::LANGUAGE.into();
        self.languages
            .insert(extensions::PYTHON.to_string(), language);
        Ok(())
    }

    /// Create a parser for a specific language
    fn create_parser(&self, extension: &str) -> Result<Option<(Parser, Language)>> {
        if let Some(language) = self.languages.get(extension).cloned() {
            let mut parser = Parser::new();
            parser
                .set_language(&language)
                .context("Failed to set language")?;
            Ok(Some((parser, language)))
        } else {
            Ok(None)
        }
    }

    /// Scan a file for @reqtrace references
    pub fn scan_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<TraceReference>> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let parser_opt = self.create_parser(extension)?;

        if parser_opt.is_none() {
            // Try regex fallback for unsupported languages
            return self.scan_file_regex(path);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Get query string first
        let query_str = self.get_comment_query(extension);

        let (mut parser, language) = parser_opt.unwrap();
        let tree = parser
            .parse(&content, None)
            .context("Failed to parse file")?;

        let mut references = Vec::new();

        // Query for comments in the language
        let query =
            Query::new(&language, query_str).context("Failed to create Tree-sitter query")?;

        let mut cursor = QueryCursor::new();

        // In tree-sitter 0.26+, QueryMatches uses StreamingIterator
        // We use for_each to process each match
        cursor
            .matches(&query, tree.root_node(), content.as_bytes())
            .for_each(|m| {
                for capture in m.captures {
                    let node = capture.node;
                    let text = &content[node.byte_range()];

                    // Extract @reqtrace references
                    if let Some(req_ids) = self.extract_reqtrace_ids(text) {
                        let line_number = node.start_position().row + 1;

                        for req_id in req_ids {
                            references.push(TraceReference {
                                req_id,
                                file_path: path.to_path_buf(),
                                line_number,
                            });
                        }
                    }
                }
            });

        Ok(references)
    }

    /// Scan multiple files in parallel using rayon
    /// Returns (all_references, errors) where errors contains failed file paths
    pub fn scan_files_parallel(
        &self,
        paths: Vec<PathBuf>,
    ) -> (Vec<TraceReference>, Vec<(PathBuf, anyhow::Error)>) {
        // TODO: Add progress reporting for large repositories
        let results: Vec<_> = paths
            .par_iter()
            .map(|path| {
                self.scan_file(path)
                    .map(|refs| (path.clone(), Ok(refs)))
                    .unwrap_or_else(|e| (path.clone(), Err(e)))
            })
            .collect();

        let mut all_references = Vec::new();
        let mut errors = Vec::new();

        for (path, result) in results {
            match result {
                Ok(refs) => all_references.extend(refs),
                Err(e) => errors.push((path, e)),
            }
        }

        (all_references, errors)
    }

    /// Get the Tree-sitter query for comments in a given language
    fn get_comment_query(&self, extension: &str) -> &str {
        match extension {
            extensions::PYTHON => queries::PYTHON,
            extensions::TYPESCRIPT | extensions::JAVASCRIPT => queries::TYPESCRIPT,
            extensions::RUST => queries::RUST,
            extensions::GO => queries::GO,
            _ => queries::DEFAULT,
        }
    }

    /// Extract @reqtrace IDs from comment text
    fn extract_reqtrace_ids(&self, text: &str) -> Option<Vec<String>> {
        let ids: Vec<String> = REQTRACE_REGEX
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();

        if ids.is_empty() { None } else { Some(ids) }
    }

    /// Fallback regex-based scanning for unsupported languages
    fn scan_file_regex<P: AsRef<Path>>(&self, path: P) -> Result<Vec<TraceReference>> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let mut references = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for cap in REQTRACE_REGEX.captures_iter(line) {
                if let Some(req_id) = cap.get(1) {
                    references.push(TraceReference {
                        req_id: req_id.as_str().to_string(),
                        file_path: path.to_path_buf(),
                        line_number: line_num + 1,
                    });
                }
            }
        }

        Ok(references)
    }
}

impl Default for CodeScanner {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default CodeScanner")
    }
}

/// Results of traceability analysis
#[derive(Debug, Serialize)]
pub struct TraceabilityReport {
    pub total_requirements: usize,
    pub covered_requirements: usize,
    pub references: Vec<TraceReference>,
    pub uncovered_ids: Vec<String>,
}

impl TraceabilityReport {
    pub fn coverage_percentage(&self) -> f64 {
        if self.total_requirements == 0 {
            return 0.0;
        }
        (self.covered_requirements as f64 / self.total_requirements as f64) * 100.0
    }

    pub fn print_summary(&self) {
        use crate::config::report::*;

        println!("\n{}", TITLE);
        println!("Total Requirements: {}", self.total_requirements);
        println!("Covered: {}", self.covered_requirements);
        println!("Coverage: {:.1}%", self.coverage_percentage());

        if !self.references.is_empty() {
            println!(
                "\n{} Connected Traces {}",
                SECTION_SEPARATOR, SECTION_SEPARATOR
            );
            for trace in &self.references {
                println!(
                    "{} {} -> {}:{}",
                    TRACE_CONNECTED_PREFIX,
                    trace.req_id,
                    trace.file_path.display(),
                    trace.line_number
                );
            }
        }

        if !self.uncovered_ids.is_empty() {
            println!(
                "\n{} Disconnected Requirements {}",
                SECTION_SEPARATOR, SECTION_SEPARATOR
            );
            for id in &self.uncovered_ids {
                println!("{} {}", TRACE_DISCONNECTED_PREFIX, id);
            }
        }
        println!();
    }
}
