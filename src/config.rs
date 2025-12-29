//! Configuration constants for ReqTrace

#![allow(dead_code)]

/// Requirement parsing constants
pub mod requirement {
    /// Frontmatter delimiter in Markdown files
    pub const FRONTMATTER_DELIMITER: &str = "---";

    /// English acceptance criterion marker
    pub const AC_MARKER_EN: &str = "AC-";

    /// Chinese acceptance criterion marker
    pub const AC_MARKER_ZH: &str = "验收";

    /// Default requirement file extension
    pub const DEFAULT_EXTENSION: &str = "md";
}

/// File discovery constants
pub mod discovery {
    /// Directories to exclude from scanning
    pub const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
        "target",
        "node_modules",
        ".git",
        "dist",
        "build",
        "__pycache__",
        ".pytest_cache",
        "coverage",
        ".next",
        ".nuxt",
        "vendor",
    ];
}

/// Scanner constants
pub mod scanner {
    /// Regex pattern for extracting @reqtrace:ID from comments
    pub const REQTRACE_PATTERN: &str = r"@reqtrace:([A-Z0-9\-\.]+)";

    /// File extensions and their language identifiers
    pub mod extensions {
        pub const PYTHON: &str = "py";
        pub const TYPESCRIPT: &str = "ts";
        pub const JAVASCRIPT: &str = "js";
        pub const RUST: &str = "rs";
        pub const GO: &str = "go";
    }

    /// Tree-sitter queries for different languages
    pub mod queries {
        /// Python comment and docstring query
        pub const PYTHON: &str = r#"
            (comment) @comment
            (string) @docstring
        "#;

        /// TypeScript/JavaScript comment query
        pub const TYPESCRIPT: &str = r#"
            (comment) @comment
        "#;

        /// Rust comment query
        pub const RUST: &str = r#"
            (line_comment) @comment
            (block_comment) @comment
        "#;

        /// Go comment query
        pub const GO: &str = r#"
            (comment) @comment
        "#;

        /// Default fallback query
        pub const DEFAULT: &str = "(comment) @comment";
    }
}

/// Report formatting constants
pub mod report {
    /// Report header
    pub const TITLE: &str = "=== ReqTrace Report ===";

    /// Connected trace prefix
    pub const TRACE_CONNECTED_PREFIX: &str = "  ✓";

    /// Disconnected trace prefix
    pub const TRACE_DISCONNECTED_PREFIX: &str = "  ✗";

    /// Section separator
    pub const SECTION_SEPARATOR: &str = "---";
}
