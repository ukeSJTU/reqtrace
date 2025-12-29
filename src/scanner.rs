use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// Reference to a requirement found in code
#[derive(Debug, Clone)]
pub struct TraceReference {
    pub req_id: String,
    pub file_path: String,
    pub line_number: usize,
    #[allow(dead_code)]
    pub context: String,
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
        self.languages.insert("py".to_string(), language);
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
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let parser_opt = self.create_parser(extension)?;
        
        if parser_opt.is_none() {
            // Try regex fallback for unsupported languages
            return self.scan_file_regex(path);
        }

        let content = fs::read_to_string(path)
            .context(format!("Failed to read file: {:?}", path))?;

        // Get query string first
        let query_str = self.get_comment_query(extension);
        
        let (mut parser, language) = parser_opt.unwrap();
        let tree = parser
            .parse(&content, None)
            .context("Failed to parse file")?;

        let mut references = Vec::new();
        
        // Query for comments in the language
        let query = Query::new(&language, &query_str)
            .context("Failed to create Tree-sitter query")?;

        let mut cursor = QueryCursor::new();
        
        // In tree-sitter 0.26+, QueryMatches uses StreamingIterator
        // We use for_each to process each match
        cursor.matches(&query, tree.root_node(), content.as_bytes())
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
                                file_path: path.display().to_string(),
                                line_number,
                                context: text.to_string(),
                            });
                        }
                    }
                }
            });

        Ok(references)
    }

    /// Scan multiple files in parallel using rayon
    /// Returns (all_references, errors) where errors contains failed file paths
    pub fn scan_files_parallel(&self, paths: Vec<PathBuf>) -> (Vec<TraceReference>, Vec<(PathBuf, anyhow::Error)>) {
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
    fn get_comment_query(&self, extension: &str) -> String {
        match extension {
            "py" => r#"
                (comment) @comment
                (string) @docstring
            "#.to_string(),
            "ts" | "js" => r#"
                (comment) @comment
            "#.to_string(),
            "rs" => r#"
                (line_comment) @comment
                (block_comment) @comment
            "#.to_string(),
            _ => "(comment) @comment".to_string(),
        }
    }

    /// Extract @reqtrace IDs from comment text
    fn extract_reqtrace_ids(&self, text: &str) -> Option<Vec<String>> {
        let re = regex::Regex::new(r"@reqtrace:([A-Z0-9\-\.]+)").ok()?;
        let ids: Vec<String> = re
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();

        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    }

    /// Fallback regex-based scanning for unsupported languages
    fn scan_file_regex<P: AsRef<Path>>(&self, path: P) -> Result<Vec<TraceReference>> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .context(format!("Failed to read file: {:?}", path))?;

        let mut references = Vec::new();
        let re = regex::Regex::new(r"@reqtrace:([A-Z0-9\-\.]+)")
            .context("Failed to create regex")?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in re.captures_iter(line) {
                if let Some(req_id) = cap.get(1) {
                    references.push(TraceReference {
                        req_id: req_id.as_str().to_string(),
                        file_path: path.display().to_string(),
                        line_number: line_num + 1,
                        context: line.to_string(),
                    });
                }
            }
        }

        Ok(references)
    }
}

/// Results of traceability analysis
#[derive(Debug)]
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
        println!("\n=== ReqTrace Report ===");
        println!("Total Requirements: {}", self.total_requirements);
        println!("Covered: {}", self.covered_requirements);
        println!("Coverage: {:.1}%", self.coverage_percentage());
        
        if !self.references.is_empty() {
            println!("\n--- Connected Traces ---");
            for trace in &self.references {
                println!("  ✓ {} -> {}:{}", trace.req_id, trace.file_path, trace.line_number);
            }
        }

        if !self.uncovered_ids.is_empty() {
            println!("\n--- Disconnected Requirements ---");
            for id in &self.uncovered_ids {
                println!("  ✗ {}", id);
            }
        }
        println!();
    }
}
