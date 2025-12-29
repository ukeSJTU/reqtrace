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

### Example 1: Single File Mode (原有方式)

```bash
cargo run -- check \
  --requirement examples/requirements/REQ-01-login-lock.md \
  --code examples/python/test_auth.py
```

Expected output:

```text
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

### Example 2: Directory Scanning (新功能)

```bash
# 扫描整个目录
cargo run -- check \
  --requirement examples/requirements/ \
  --code examples/python/
```

Expected output:

```
Discovering requirement files from: examples/requirements/
Found 1 requirement file(s)
Loaded 1 requirement(s) successfully
  - REQ-01: 登录暴力破解防护

Discovering code files...
Found 2 code file(s)

Scanning code files in parallel...
Found 6 trace reference(s)

=== ReqTrace Report ===
Total Requirements: 3
Covered: 3
Coverage: 100.0%

--- Connected Traces ---
  ✓ REQ-01 -> examples/python/test_auth.py:13
  ✓ REQ-01.AC-01 -> examples/python/test_auth.py:19
  ✓ REQ-01.AC-02 -> examples/python/test_auth.py:41
  ✓ REQ-01.AC-01 -> examples/python/test_auth.py:58
  ✓ REQ-01 -> examples/python/test_incomplete.py:9
  ✓ REQ-01.AC-01 -> examples/python/test_incomplete.py:15

All requirements are properly traced!
```

### Example 3: Glob Pattern Matching (新功能)

```bash
# 使用通配符模式精确控制扫描范围
cargo run -- check \
  --requirement "examples/requirements/*.md" \
  --code "examples/**/*.py"
```

这会产生与 Example 2 相同的结果，但你可以更灵活地控制：

-   `*.md` - 只扫描当前目录的 .md 文件
-   `**/*.md` - 递归扫描所有 .md 文件
-   `tests/**/*.py` - 只扫描 tests 目录下的 Python 文件
-   `src/**/test_*.py` - 只扫描以 test\_ 开头的 Python 文件

### Example 4: Mixed Input Types (新功能)

```bash
# 混合使用文件、目录和模式
cargo run -- check \
  --requirement requirements/ \
  --code examples/python/test_auth.py "tests/**/*.py" src/
```

ReqTrace 会智能识别：

-   单个文件 → 直接处理
-   目录 → 递归扫描所有文件
-   含通配符 → 使用 glob 模式匹配

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

1. **Requirement documents** (Markdown with frontmatter) to extract:

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

## TODO: Future Enhancements

-   [ ] Default exclude patterns (target/, node_modules/, .git/, etc.)
-   [ ] Progress indicator for large repository scans
-   [ ] Custom exclude patterns via config file
-   [ ] Watch mode for development
