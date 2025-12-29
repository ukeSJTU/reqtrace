# ReqTrace Examples

This directory contains example files demonstrating how ReqTrace works.

## Directory Structure

```
examples/
├── requirements/       # Requirement markdown files
│   └── REQ-01-login-lock.md
└── python/            # Python test files
    ├── test_auth.py           # Full coverage (100%)
    └── test_incomplete.py     # Incomplete coverage (66.7%)
```

## Running Examples

### Full Coverage Example
```bash
cargo run -- check --requirement examples/requirements/REQ-01-login-lock.md --code examples/python/test_auth.py
```

Expected output:
```
Parsing requirement: examples/requirements/REQ-01-login-lock.md
   ID: REQ-01
   Title: 登录暴力破解防护
   Hash: 726ca6203b465178
   Acceptance Criteria: 2
     - REQ-01.AC-01: AC-01: 触发锁定
     - REQ-01.AC-02: AC-02: 锁定期间尝试

Scanning code files...
   Scanning: examples/python/test_auth.py
     Found 4 references

=== ReqTrace Report ===
Total Requirements: 3
Covered: 3
Coverage: 100.0%

--- Connected Traces ---
  ✓ REQ-01 -> examples/python/test_auth.py:13
  ✓ REQ-01.AC-01 -> examples/python/test_auth.py:19
  ✓ REQ-01.AC-02 -> examples/python/test_auth.py:41
  ✓ REQ-01.AC-01 -> examples/python/test_auth.py:58

All requirements are properly traced!
```

### Incomplete Coverage Example
```bash
cargo run -- check --requirement examples/requirements/REQ-01-login-lock.md --code examples/python/test_incomplete.py
```

Expected output:
```
Parsing requirement: examples/requirements/REQ-01-login-lock.md
   ID: REQ-01
   Title: 登录暴力破解防护
   Hash: 726ca6203b465178
   Acceptance Criteria: 2
     - REQ-01.AC-01: AC-01: 触发锁定
     - REQ-01.AC-02: AC-02: 锁定期间尝试

Scanning code files...
   Scanning: examples/python/test_incomplete.py
     Found 2 references

=== ReqTrace Report ===
Total Requirements: 3
Covered: 2
Coverage: 66.7%

--- Connected Traces ---
  ✓ REQ-01 -> examples/python/test_incomplete.py:9
  ✓ REQ-01.AC-01 -> examples/python/test_incomplete.py:15

--- Disconnected Requirements ---
  ✗ REQ-01.AC-02

Not all requirements are traced!
```

The command will exit with code 1, which can be used in CI/CD pipelines to block incomplete coverage.

## How It Works

ReqTrace scans:
1. **Requirement document** (Markdown with frontmatter) to extract:
   - Requirement ID and metadata
   - Acceptance criteria (### headings)
   - Content hash for staleness detection

2. **Source code files** using Tree-sitter to find:
   - `@reqtrace:REQ-ID` references in comments/docstrings
   - File location and line numbers

3. **Traceability analysis**:
   - Matches code references to requirements
   - Reports coverage percentage
   - Identifies disconnected requirements
   - Can be used as a CI gate
