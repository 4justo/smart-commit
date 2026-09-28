use crate::diff::StructureInfo;
use std::path::{Path, PathBuf};

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

pub fn parse_file_structures(path: &Path, content: &str) -> Option<Vec<StructureInfo>> {
    let language = detect_language(path);
    let file_path = path.to_path_buf();
    
    match language.as_str() {
        "rust" => parse_rust_structures(content, file_path),
        "python" => parse_python_structures(content, file_path),
        "javascript" | "typescript" => parse_javascript_structures(content, file_path),
        "php" => parse_php_structures(content, file_path),
        "java" => parse_java_structures(content, file_path),
        "csharp" => parse_csharp_structures(content, file_path),
        "go" => parse_go_structures(content, file_path),
        "c" => parse_c_structures(content, file_path),
        "cpp" => parse_cpp_structures(content, file_path),
        _ => None,
    }
}

pub fn find_structure_at_line(structures: &[StructureInfo], line: usize) -> Option<&StructureInfo> {
    structures
        .iter()
        .find(|s| line >= s.start_line && line <= s.end_line)
}

#[cfg(feature = "tree-sitter-rust")]
fn parse_rust_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_rust_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-rust"))]
fn parse_rust_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-rust")]
fn extract_rust_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = extract_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                    file: file_path.clone(),
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
                    file: file_path.clone(),
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
                    file: file_path.clone(),
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
                    file: file_path.clone(),
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
                    file: file_path.clone(),
                });
            }
        }
        "impl_item" => {
            if let Some(name) = extract_impl_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Struct,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "rust".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_rust_nodes(&child, source, structures, file_path.clone());
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

#[cfg(feature = "tree-sitter-python")]
fn parse_python_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_python_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-python"))]
fn parse_python_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-python")]
fn extract_python_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_python_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "python".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "class_definition" => {
            if let Some(name) = extract_python_class_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Class,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "python".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "module" => {
            if let Some(name) = extract_python_module_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Module,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "python".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_python_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-python")]
fn extract_python_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-python")]
fn extract_python_class_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-python")]
fn extract_python_module_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "dotted_name" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn parse_javascript_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    
    #[cfg(feature = "tree-sitter-typescript")]
    let lang_name = if parser.set_language(&tree_sitter_typescript::language_typescript()).is_ok() {
        "typescript"
    } else {
        #[cfg(feature = "tree-sitter-javascript")]
        {
            parser.set_language(&tree_sitter_javascript::language()).ok()?;
            "javascript"
        }
        #[cfg(not(feature = "tree-sitter-javascript"))]
        {
            return None;
        }
    };
    
    #[cfg(not(feature = "tree-sitter-typescript"))]
    let lang_name = {
        #[cfg(feature = "tree-sitter-javascript")]
        {
            parser.set_language(&tree_sitter_javascript::language()).ok()?;
            "javascript"
        }
        #[cfg(not(feature = "tree-sitter-javascript"))]
        {
            return None;
        }
    };
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_javascript_nodes(&root_node, content, &mut structures, lang_name, file_path);
    
    Some(structures)
}

#[cfg(not(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript")))]
fn parse_javascript_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn extract_javascript_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, lang_name: &str, file_path: PathBuf) {
    match node.kind() {
        "function_declaration" | "function_expression" | "arrow_function" => {
            if let Some(name) = extract_javascript_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: lang_name.to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "class_declaration" | "class_expression" => {
            if let Some(name) = extract_javascript_class_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Class,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: lang_name.to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "method_definition" => {
            if let Some(name) = extract_javascript_method_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Method,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: lang_name.to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name) = extract_javascript_interface_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Interface,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: lang_name.to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "import_statement" | "export_statement" => {
            if let Some(name) = extract_javascript_import_export_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: if node.kind() == "import_statement" {
                        StructureKind::Import
                    } else {
                        StructureKind::Export
                    },
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: lang_name.to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_javascript_nodes(&child, source, structures, lang_name, file_path.clone());
    }
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn extract_javascript_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn extract_javascript_class_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn extract_javascript_method_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "property_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn extract_javascript_interface_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(any(feature = "tree-sitter-javascript", feature = "tree-sitter-typescript"))]
fn extract_javascript_import_export_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "string" || child.kind() == "identifier" {
            let text = source[child.byte_range()].to_string();
            if text.starts_with('"') || text.starts_with('\'') {
                return Some(text.trim_matches('"').trim_matches('\'').to_string());
            }
            return Some(text);
        }
    }
    None
}

#[cfg(feature = "tree-sitter-php")]
fn parse_php_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_php::language_php())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_php_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-php"))]
fn parse_php_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-php")]
fn extract_php_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_php_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "php".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "class_declaration" => {
            if let Some(name) = extract_php_class_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Class,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "php".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "method_declaration" => {
            if let Some(name) = extract_php_method_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Method,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "php".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name) = extract_php_interface_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Interface,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "php".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_php_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-php")]
fn extract_php_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "name" {
            return Some(source[child.byte_range()].to_string());
        }
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-php")]
fn extract_php_class_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "name" {
            return Some(source[child.byte_range()].to_string());
        }
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-php")]
fn extract_php_method_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "name" {
            return Some(source[child.byte_range()].to_string());
        }
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-php")]
fn extract_php_interface_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "name" {
            return Some(source[child.byte_range()].to_string());
        }
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-java")]
fn parse_java_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_java_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-java"))]
fn parse_java_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-java")]
fn extract_java_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "method_declaration" => {
            if let Some(name) = extract_java_method_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Method,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "java".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "class_declaration" => {
            if let Some(name) = extract_java_class_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Class,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "java".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name) = extract_java_interface_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Interface,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "java".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_java_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-java")]
fn extract_java_method_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-java")]
fn extract_java_class_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-java")]
fn extract_java_interface_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-c-sharp")]
fn parse_csharp_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_csharp_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-c-sharp"))]
fn parse_csharp_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-c-sharp")]
fn extract_csharp_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "method_declaration" => {
            if let Some(name) = extract_csharp_method_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Method,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "csharp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "class_declaration" => {
            if let Some(name) = extract_csharp_class_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Class,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "csharp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name) = extract_csharp_interface_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Interface,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "csharp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_csharp_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-c-sharp")]
fn extract_csharp_method_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-c-sharp")]
fn extract_csharp_class_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-c-sharp")]
fn extract_csharp_interface_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-go")]
fn parse_go_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_go_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-go"))]
fn parse_go_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-go")]
fn extract_go_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = extract_go_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "go".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "method_declaration" => {
            if let Some(name) = extract_go_method_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Method,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "go".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "type_declaration" => {
            if let Some(name) = extract_go_type_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Struct,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "go".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name) = extract_go_interface_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Interface,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "go".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_go_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-go")]
fn extract_go_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-go")]
fn extract_go_method_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-go")]
fn extract_go_struct_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-go")]
fn extract_go_interface_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-c")]
fn parse_c_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_c_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-c"))]
fn parse_c_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-c")]
fn extract_c_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_c_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "c".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "struct_specifier" => {
            if let Some(name) = extract_c_struct_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Struct,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "c".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "enum_specifier" => {
            if let Some(name) = extract_c_enum_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Enum,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "c".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_c_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-c")]
fn extract_c_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
        if child.kind() == "function_declarator" || child.kind() == "declarator" {
            for subchild in child.children(&mut child.walk()) {
                if subchild.kind() == "identifier" {
                    return Some(source[subchild.byte_range()].to_string());
                }
            }
        }
    }
    None
}

#[cfg(feature = "tree-sitter-c")]
fn extract_c_struct_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-c")]
fn extract_c_enum_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-cpp")]
fn parse_cpp_structures(content: &str, file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    use tree_sitter::Parser;
    
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::language())
        .ok()?;
    
    let tree = parser.parse(content, None)?;
    let root_node = tree.root_node();
    
    let mut structures = Vec::new();
    extract_cpp_nodes(&root_node, content, &mut structures, file_path);
    
    Some(structures)
}

#[cfg(not(feature = "tree-sitter-cpp"))]
fn parse_cpp_structures(_content: &str, _file_path: PathBuf) -> Option<Vec<StructureInfo>> {
    None
}

#[cfg(feature = "tree-sitter-cpp")]
fn extract_cpp_nodes(node: &tree_sitter::Node, source: &str, structures: &mut Vec<StructureInfo>, file_path: PathBuf) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_cpp_function_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Function,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "cpp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "class_specifier" => {
            if let Some(name) = extract_cpp_class_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Class,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "cpp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "struct_specifier" => {
            if let Some(name) = extract_cpp_struct_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Struct,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "cpp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        "enum_specifier" => {
            if let Some(name) = extract_cpp_enum_name(node, source) {
                structures.push(StructureInfo {
                    name,
                    kind: StructureKind::Enum,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    language: "cpp".to_string(),
                    file: file_path.clone(),
                });
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_cpp_nodes(&child, source, structures, file_path.clone());
    }
}

#[cfg(feature = "tree-sitter-cpp")]
fn extract_cpp_function_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-cpp")]
fn extract_cpp_class_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-cpp")]
fn extract_cpp_struct_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(feature = "tree-sitter-cpp")]
fn extract_cpp_enum_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::diff::StructureKind;
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

    #[cfg(feature = "tree-sitter-rust")]
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

    #[cfg(feature = "tree-sitter-rust")]
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

    #[cfg(feature = "tree-sitter-rust")]
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

    #[cfg(feature = "tree-sitter-rust")]
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

    #[cfg(feature = "tree-sitter-rust")]
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
                file: PathBuf::from("test.rs"),
            },
            StructureInfo {
                name: "func2".to_string(),
                kind: StructureKind::Function,
                start_line: 15,
                end_line: 25,
                language: "rust".to_string(),
                file: PathBuf::from("test.rs"),
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

    #[cfg(feature = "tree-sitter-python")]
    #[test]
    fn test_parse_python_function() {
        let code = r#"
def validate_token(token: str) -> bool:
    return len(token) > 0
"#;
        
        let structures = parse_python_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validate_token");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "python");
    }

    #[cfg(feature = "tree-sitter-python")]
    #[test]
    fn test_parse_python_class() {
        let code = r#"
class User:
    def __init__(self, name: str):
        self.name = name
"#;
        
        let structures = parse_python_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let class_info = &structures[0];
        assert_eq!(class_info.name, "User");
        assert_eq!(class_info.kind, StructureKind::Class);
        assert_eq!(class_info.language, "python");
    }

    #[cfg(feature = "tree-sitter-javascript")]
    #[test]
    fn test_parse_javascript_function() {
        let code = r#"
function validateToken(token) {
    return token.length > 0;
}
"#;
        
        let structures = parse_javascript_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validateToken");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "javascript");
    }

    #[cfg(feature = "tree-sitter-javascript")]
    #[test]
    fn test_parse_javascript_class() {
        let code = r#"
class User {
    constructor(name) {
        this.name = name;
    }
}
"#;
        
        let structures = parse_javascript_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let class_info = &structures[0];
        assert_eq!(class_info.name, "User");
        assert_eq!(class_info.kind, StructureKind::Class);
        assert_eq!(class_info.language, "javascript");
    }

    #[cfg(feature = "tree-sitter-typescript")]
    #[test]
    fn test_parse_typescript_function() {
        let code = r#"
function validateToken(token: string): boolean {
    return token.length > 0;
}
"#;
        
        let structures = parse_javascript_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validateToken");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "typescript");
    }

    #[cfg(feature = "tree-sitter-typescript")]
    #[test]
    fn test_parse_typescript_interface() {
        let code = r#"
interface User {
    name: string;
    email: string;
}
"#;
        
        let structures = parse_javascript_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let interface_info = &structures[0];
        assert_eq!(interface_info.name, "User");
        assert_eq!(interface_info.kind, StructureKind::Interface);
        assert_eq!(interface_info.language, "typescript");
    }

    #[cfg(feature = "tree-sitter-php")]
    #[test]
    fn test_parse_php_function() {
        let code = r#"
<?php
function validateToken($token) {
    return strlen($token) > 0;
}
"#;
        
        let structures = parse_php_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validateToken");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "php");
    }

    #[cfg(feature = "tree-sitter-php")]
    #[test]
    fn test_parse_php_class() {
        let code = r#"
<?php
class User {
    public $name;
    
    public function __construct($name) {
        $this->name = $name;
    }
}
"#;
        
        let structures = parse_php_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let class_info = &structures[0];
        assert_eq!(class_info.name, "User");
        assert_eq!(class_info.kind, StructureKind::Class);
        assert_eq!(class_info.language, "php");
    }

    #[cfg(feature = "tree-sitter-php")]
    #[test]
    fn test_parse_php_interface() {
        let code = r#"
<?php
interface Validator {
    public function validate($value): bool;
}
"#;
        
        let structures = parse_php_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let interface_info = &structures[0];
        assert_eq!(interface_info.name, "Validator");
        assert_eq!(interface_info.kind, StructureKind::Interface);
        assert_eq!(interface_info.language, "php");
    }

    #[cfg(feature = "tree-sitter-java")]
    #[test]
    fn test_parse_java_class() {
        let code = r#"
public class User {
    private String name;
    
    public User(String name) {
        this.name = name;
    }
}
"#;
        
        let structures = parse_java_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let class_info = &structures[0];
        assert_eq!(class_info.name, "User");
        assert_eq!(class_info.kind, StructureKind::Class);
        assert_eq!(class_info.language, "java");
    }

    #[cfg(feature = "tree-sitter-c-sharp")]
    #[test]
    fn test_parse_csharp_class() {
        let code = r#"
public class User {
    private string name;
    
    public User(string name) {
        this.name = name;
    }
}
"#;
        
        let structures = parse_csharp_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let class_info = &structures[0];
        assert_eq!(class_info.name, "User");
        assert_eq!(class_info.kind, StructureKind::Class);
        assert_eq!(class_info.language, "csharp");
    }

    #[cfg(feature = "tree-sitter-go")]
    #[test]
    fn test_parse_go_function() {
        let code = r#"
package main

func validateToken(token string) bool {
    return len(token) > 0
}
"#;
        
        let structures = parse_go_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validateToken");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "go");
    }

    #[cfg(feature = "tree-sitter-c")]
    #[test]
    fn test_parse_c_function() {
        let code = r#"
#include <stdio.h>

int validate_token(const char* token) {
    return strlen(token) > 0;
}
"#;
        
        let structures = parse_c_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let func = &structures[0];
        assert_eq!(func.name, "validate_token");
        assert_eq!(func.kind, StructureKind::Function);
        assert_eq!(func.language, "c");
    }

    #[cfg(feature = "tree-sitter-cpp")]
    #[test]
    fn test_parse_cpp_class() {
        let code = r#"
class User {
private:
    std::string name;
public:
    User(std::string name) : name(name) {}
};
"#;
        
        let structures = parse_cpp_structures(code);
        assert!(structures.is_some());
        let structures = structures.unwrap();
        assert!(!structures.is_empty());
        
        let class_info = &structures[0];
        assert_eq!(class_info.name, "User");
        assert_eq!(class_info.kind, StructureKind::Class);
        assert_eq!(class_info.language, "cpp");
    }
}
