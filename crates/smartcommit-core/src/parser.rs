//! Code structure parsing using Tree-sitter (Module 18).
//!
//! This module detects code structures (functions, classes, structs, etc.)
//! in source files to provide context for diff hunks.

use crate::diff::{StructureInfo, StructureKind};
use std::path::Path;

/// Detect the programming language from a file extension.
pub fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust".to_string(),
        Some("py") => "python".to_string(),
        Some("js") => "javascript".to_string(),
        Some("jsx") => "javascript".to_string(),
        Some("ts") => "typescript".to_string(),
        Some("tsx") => "typescript".to_string(),
        Some("php") => "php".to_string(),
        Some("java") => "java".to_string(),
        Some("cs") => "csharp".to_string(),
        Some("go") => "go".to_string(),
        Some("c") => "c".to_string(),
        Some("h") => "c".to_string(),
        Some("cpp") => "cpp".to_string(),
        Some("hpp") => "cpp".to_string(),
        Some("cc") => "cpp".to_string(),
        Some("cxx") => "cpp".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Parse a file and extract all code structures.
///
/// Returns None if the language is not supported or parsing fails.
pub fn parse_file_structures(path: &Path, content: &str) -> Option<Vec<StructureInfo>> {
    let language = detect_language(path);
    
    match language.as_str() {
        "rust" => parse_rust_structures(content),
        "python" => parse_python_structures(content),
        "javascript" | "typescript" => parse_javascript_structures(content),
        "php" => parse_php_structures(content),
        "java" => parse_java_structures(content),
        "csharp" => parse_csharp_structures(content),
        "go" => parse_go_structures(content),
        "c" | "cpp" => parse_c_structures(content),
        _ => None,
    }
}

/// Find the structure that contains a given line number.
pub fn find_structure_at_line(structures: &[StructureInfo], line: usize) -> Option<&StructureInfo> {
    structures
        .iter()
        .find(|s| line >= s.start_line && line <= s.end_line)
}

// Rust parser implementation
#[cfg(feature = "tree-sitter-rust")]
fn parse_rust_structures(content: &str) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_rust_nodes(&root_node, content, &mut structures);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-rust"))]
fn parse_rust_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // Fallback: return None if tree-sitter-rust is not enabled
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_rust_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = extract_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                });
            }
        }
        "struct_item" => {
            if let Some(name) = extract_struct_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Struct,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                });
            }
        }
        "enum_item" => {
            if let Some(name) = extract_enum_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Enum,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                });
            }
        }
        "trait_item" => {
            if let Some(name) = extract_trait_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Trait,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                });
            }
        }
        "mod_item" => {
            if let Some(name) = extract_mod_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Module,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                });
            }
        }
        "impl_item" => {
            if let Some(name) = extract_impl_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Struct, // impl blocks are associated with structs
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                });
            }
        }
        _ => {}
    }
    
    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_rust_nodes(&child, source, structures);
    }
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_struct_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_enum_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_trait_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_mod_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_impl_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

// Placeholder implementations for other languages
// These will be implemented in subsequent sections

fn parse_python_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement Python parser in Section 2
    None
}

fn parse_javascript_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement JavaScript/TypeScript parser in Section 3
    None
}

fn parse_php_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement PHP parser in Section 4
    None
}

fn parse_java_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement Java parser in Section 5
    None
}

fn parse_csharp_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement C# parser in Section 6
    None
}

fn parse_go_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement Go parser in Section 7
    None
}

fn parse_c_structures(_content: &str) -> Option<Vec<StructureInfo>> {
    // TODO: Implement C/C++ parser in Section 8
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("test.rs")), "rust");
        assert_eq!(detect_language(Path::new("test.py")), "python");
        assert_eq!(detect_language(Path::new("test.js")), "javascript");
        assert_eq!(detect_language(Path::new("test.ts")), "typescript");
        assert_eq!(detect_language(Path::new("test.php")), "php");
        assert_eq!(detect_language(Path::new("test.java")), "java");
        assert_eq!(detect_language(Path::new("test.cs")), "csharp");
        assert_eq!(detect_language(Path::new("test.go")), "go");
        assert_eq!(detect_language(Path::new("test.c")), "c");
        assert_eq!(detect_language(Path::new("test.cpp")), "cpp");
        assert_eq!(detect_language(Path::new("test.unknown")), "unknown");
    }

    #[test]
    fn test_parse_rust_function() {
        let code = r#"
fn validate_token(token: &str) -> bool {
    token.len() > 0
}
"#;
        
        let structures = parse_rust_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validate_token");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "rust");
    }

    #[test]
    fn test_parse_rust_struct() {
        let code = r#"
struct User {
    name: String,
    email: String,
}
"#;
        
        let structures = parse_rust_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let struct_info = &structures[0];
        assert_eq!(struct_info.name, "User");
        assert_eq!(struct_info.kind, StructureKind::Struct);
        assert_eq!(struct_info.language, "rust");
    }

    #[test]
    fn test_parse_rust_enum() {
        let code = r#"
enum Status {
    Active,
    Inactive,
}
"#;
        
        let structures = parse_rust_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let enum_info = &structures[0];
        assert_eq!(enum_info.name, "Status");
        assert_eq!(enum_info.kind, StructureKind::Enum);
        assert_eq!(enum_info.language, "rust");
    }

    #[test]
    fn test_parse_rust_trait() {
        let code = r#"
trait Validator {
    fn validate(&self) -> bool;
}
"#;
        
        let structures = parse_rust_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let trait_info = &structures[0];
        assert_eq!(trait_info.name, "Validator");
        assert_eq!(trait_info.kind, StructureKind::Trait);
        assert_eq!(trait_info.language, "rust");
    }

    #[test]
    fn test_parse_rust_module() {
        let code = r#"
mod auth {
    fn login() {}
}
"#;
        
        let structures = parse_rust_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let mod_info = &structures[0];
        assert_eq!(mod_info.name, "auth");
        assert_eq!(mod_info.kind, StructureKind::Module);
        assert_eq!(mod_info.language, "rust");
    }

    #[test]
    fn test_find_structure_at_line() {
        let structures = vec![
            StructureInfo {
                name: "func1".to_string(),
                kind: StructureKind::Function,
                start_line: 1,
                end_line: 10,
                language: "rust".to_string(),
            },
            StructureInfo {
                name: "func2".to_string(),
                kind: StructureKind::Function,
                start_line: 15,
                end_line: 25,
                language: "rust".to_string(),
            },
        ];
        
        assert_eq!(
            find_structure_at_line(&structures, 5).map(|s|&s.name),
            Some(&"func1".to_string())
        );
        assert_eq!(
            find_structure_at_line(&structures, 20).map(|s|&s.name),
            Some(&"func2".to_string())
        );
        assert!(find_structure_at_line(&structures, 30).is_none());
    }
}
