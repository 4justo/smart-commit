# Module 18: Code Structure Detection

## Overview

SmartCommit now understands code structure within changed files, providing granular context for diff hunks. Instead of只知道 "auth.rs changed", it now knows "auth.rs → validate_token() → hunk #3".

## Features

### Supported Languages

SmartCommit supports code structure detection for the following programming languages:

- **Rust** - functions, structs, enums, traits, modules
- **Python** - functions, classes, modules
- **JavaScript** - functions, classes, methods, interfaces, imports, exports
- **TypeScript** - functions, classes, methods, interfaces, imports, exports
- **PHP** - functions, classes, methods, interfaces
- **Java** - methods, classes, interfaces
- **C#** - methods, classes, interfaces
- **Go** - functions, methods, structs, interfaces
- **C** - functions, structs, enums
- **C++** - functions, classes, structs, enums

### Detected Structure Types

- **Function** - Standalone functions
- **Method** - Class/object methods
- **Class** - Class definitions
- **Struct** - Struct definitions
- **Enum** - Enum definitions
- **Interface** - Interface definitions
- **Module** - Module definitions
- **Import** - Import statements
- **Export** - Export statements
- **Trait** - Trait definitions (Rust)

## How It Works

### Integration with Diff Parsing

When SmartCommit processes staged changes:

1. It reads the file content for each changed file
2. Uses Tree-sitter to parse the code structure
3. Associates each diff hunk with the containing code structure
4. Includes structure information in AI prompts for better commit messages

### Example Output

```
Hunk #4
File: src/auth.rs
Structure: validate_token()
Language: Rust
```

## Installation

### Building with Language Support

Language parsers are optional features. Enable them in your Cargo.toml:

```toml
[dependencies]
smartcommit-core = { version = "0.1", features = [
    "tree-sitter-rust",
    "tree-sitter-python",
    "tree-sitter-javascript",
    "tree-sitter-typescript",
    "tree-sitter-php",
    "tree-sitter-java",
    "tree-sitter-c-sharp",
    "tree-sitter-go",
    "tree-sitter-c",
    "tree-sitter-cpp"
] }
```

### Enable Individual Languages

You can enable only the languages you need:

```toml
# Only Rust and Python support
smartcommit-core = { version = "0.1", features = ["tree-sitter-rust", "tree-sitter-python"] }
```

## Usage

### Command Line

```bash
# SmartCommit automatically detects and uses enabled language parsers
smartcommit
```

### Programmatic Usage

```rust
use smartcommit_core::{get_staged_diffs, parse_file_structures};

// Get staged diffs with structure information
let diffs = get_staged_diffs("/path/to/repo")?;

// Each DiffHunk now contains optional structure info
for diff in diffs {
    for hunk in diff.hunks {
        if let Some(structure) = hunk.structure {
            println!("Changed in: {} ({})", structure.name, structure.kind);
        }
    }
}
```

## API Reference

### StructureInfo

```rust
pub struct StructureInfo {
    /// The name of the structure (e.g., "validate_token", "User", "auth")
    pub name: String,
    /// The kind of structure
    pub kind: StructureKind,
    /// The starting line number (1-based)
    pub start_line: usize,
    /// The ending line number (1-based)
    pub end_line: usize,
    /// The language of the file
    pub language: String,
}
```

### StructureKind

```rust
pub enum StructureKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    Import,
    Export,
    Trait,
    Unknown,
}
```

### Parser Functions

```rust
// Parse structures from file content
pub fn parse_file_structures(file_path: &Path, content: &str) -> Option<Vec<StructureInfo>>

// Find structure at a specific line
pub fn find_structure_at_line(structures: &[StructureInfo], line: usize) -> Option<&StructureInfo>

// Detect language from file extension
pub fn detect_language(file_path: &Path) -> &'static str
```

## Performance Considerations

- Tree-sitter parsers are compiled as native code for performance
- Parsers are optional and only loaded when the corresponding feature is enabled
- Structure detection is performed only for files with staged changes
- Caching is not currently implemented but could be added for large repositories

## Limitations

- Only supports the 10 languages listed above
- Structure detection is based on syntax parsing, not semantic analysis
- Nested structures are detected but not fully traversed (only the immediate containing structure is reported)
- Very large files may have parsing overhead

## Future Enhancements

Potential improvements for future versions:

- [ ] Add support for more languages (Ruby, Swift, Kotlin, etc.)
- [ ] Implement caching for parsed structures
- [ ] Add semantic analysis for better context
- [ ] Support for detecting nested structure hierarchies
- [ ] Integration with language servers for enhanced accuracy
- [ ] Configuration options for custom structure detection rules

## Technical Details

### Tree-sitter Integration

SmartCommit uses Tree-sitter, a parser generator tool, for robust syntax parsing across multiple languages. Each language parser is:

- Implemented as a separate optional dependency
- Conditionally compiled based on feature flags
- Provides a consistent API for structure extraction

### Architecture

```
diff.rs (DiffHunk with structure field)
    ↓
parser.rs (Language-specific parsers)
    ↓
Tree-sitter (Syntax parsing)
    ↓
StructureInfo (Detected structures)
    ↓
message_generator.rs (AI prompts with structure context)
```

## Troubleshooting

### Parser Not Working

If structure detection isn't working for a language:

1. Ensure the language feature is enabled in Cargo.toml
2. Verify the file extension matches the expected language
3. Check that the file contains valid syntax for the language

### Performance Issues

If parsing is slow:

1. Enable only the language features you need
2. Consider excluding large generated files from git staging
3. Check if the repository has unusually large files

## Testing

Run tests for specific language parsers:

```bash
# Test Rust parser
cargo test --package smartcommit-core --features tree-sitter-rust parser

# Test Python parser
cargo test --package smartcommit-core --features tree-sitter-python parser

# Test all enabled parsers
cargo test --package smartcommit-core parser
```

## Contributing

To add support for a new language:

1. Add the Tree-sitter dependency to Cargo.toml
2. Implement `parse_<language>_structures()` in parser.rs
3. Implement extraction functions for relevant structure types
4. Add tests for the new parser
5. Update documentation

## License

This module is part of SmartCommit and follows the same license terms.
