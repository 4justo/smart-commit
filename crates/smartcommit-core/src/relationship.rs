use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipType {
    TestCoverage,
    Import,
    SharedDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSource {
    Deterministic(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub from: PathBuf,
    pub to: PathBuf,
    pub relationship_type: RelationshipType,
    pub evidence: EvidenceSource,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default)]
pub struct RelationshipGraph {
    pub nodes: Vec<PathBuf>,
    pub edges: Vec<Relationship>,
}

impl RelationshipGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, path: PathBuf) {
        if !self.nodes.contains(&path) {
            self.nodes.push(path);
        }
    }

    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.add_node(relationship.from.clone());
        self.add_node(relationship.to.clone());
        self.edges.push(relationship);
    }

    pub fn find_related_files(&self, path: &Path) -> Vec<&PathBuf> {
        self.edges
            .iter()
            .filter(|r| r.from == path || r.to == path)
            .map(|r| {
                if r.from == path {
                    &r.to
                } else {
                    &r.from
                }
            })
            .collect()
    }
}

pub fn find_import_relationships(files: &[PathBuf], repo_root: &Path) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    for file in files {
        if let Some(content) = std::fs::read_to_string(repo_root.join(file)).ok() {
            let imports = extract_imports(file, &content);
            for import in imports {
                for target_file in files {
                    if file_matches_import(target_file, &import, repo_root) {
                        if target_file != file {
                            relationships.push(Relationship {
                                from: file.clone(),
                                to: target_file.clone(),
                                relationship_type: RelationshipType::Import,
                                evidence: EvidenceSource::Deterministic(format!(
                                    "{} imports {}",
                                    file.display(),
                                    import
                                )),
                                confidence: 0.95,
                            });
                        }
                    }
                }
            }
        }
    }

    relationships
}

fn extract_imports(file: &Path, content: &str) -> Vec<String> {
    let language = crate::parser::detect_language(file);
    
    match language.as_str() {
        "rust" => extract_rust_imports(content),
        "python" => extract_python_imports(content),
        "javascript" | "typescript" => extract_javascript_imports(content),
        _ => Vec::new(),
    }
}

fn extract_rust_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("use ") {
            let import = line.strip_prefix("use ").unwrap();
            let import = import.trim_end_matches(';');
            imports.push(import.to_string());
        } else if line.starts_with("mod ") {
            let mod_name = line.strip_prefix("mod ").unwrap();
            let mod_name = mod_name.split_whitespace().next().unwrap_or("");
            if !mod_name.is_empty() {
                imports.push(mod_name.to_string());
            }
        }
    }
    
    imports
}

fn extract_python_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("import ") {
            let import = line.strip_prefix("import ").unwrap();
            let import = import.split(',').next().unwrap_or("").trim();
            if !import.is_empty() {
                imports.push(import.to_string());
            }
        } else if line.starts_with("from ") {
            let import = line.strip_prefix("from ").unwrap();
            let import = import.split(" import ").next().unwrap_or("").trim();
            if !import.is_empty() {
                imports.push(import.to_string());
            }
        }
    }
    
    imports
}

fn extract_javascript_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("import ") {
            if let Some(start) = line.find("from ") {
                let import = &line[start + 5..];
                let import = import.trim().trim_matches('"').trim_matches('\'').trim_matches('`');
                if !import.is_empty() && !import.starts_with('.') {
                    imports.push(import.to_string());
                }
            }
        } else if line.starts_with("require(") {
            let import = line.strip_prefix("require(").unwrap();
            let import = import.trim_end_matches(')');
            let import = import.trim().trim_matches('"').trim_matches('\'').trim_matches('`');
            if !import.is_empty() && !import.starts_with('.') {
                imports.push(import.to_string());
            }
        }
    }
    
    imports
}

fn file_matches_import(file: &Path, import: &str, _repo_root: &Path) -> bool {
    let file_name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_name_lower = file_name.to_lowercase();
    let import_lower = import.to_lowercase();
    
    if file_name_lower == import_lower {
        return true;
    }
    
    if let Some(parent) = file.parent() {
        let parent_name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if parent_name.to_lowercase() == import_lower {
            return true;
        }
    }
    
    let import_path = Path::new(import);
    if let Some(import_stem) = import_path.file_stem().and_then(|s| s.to_str()) {
        if file_name_lower == import_stem.to_lowercase() {
            return true;
        }
    }
    
    false
}

pub fn find_shared_directory_relationships(files: &[PathBuf]) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    for (i, file_a) in files.iter().enumerate() {
        for file_b in files.iter().skip(i + 1) {
            if file_a == file_b {
                continue;
            }

            if let Some(parent_a) = file_a.parent() {
                if let Some(parent_b) = file_b.parent() {
                    if parent_a == parent_b {
                        relationships.push(Relationship {
                            from: file_a.clone(),
                            to: file_b.clone(),
                            relationship_type: RelationshipType::SharedDirectory,
                            evidence: EvidenceSource::Deterministic(format!(
                                "Both files in directory: {}",
                                parent_a.display()
                            )),
                            confidence: 0.7,
                        });
                    } else if let Some(grandparent_a) = parent_a.parent() {
                        if let Some(grandparent_b) = parent_b.parent() {
                            if grandparent_a == grandparent_b {
                                let dir_a = parent_a.file_name().and_then(|s| s.to_str()).unwrap_or("");
                                let dir_b = parent_b.file_name().and_then(|s| s.to_str()).unwrap_or("");
                                
                                if (dir_a == "tests" && dir_b != "tests") || (dir_b == "tests" && dir_a != "tests") {
                                    relationships.push(Relationship {
                                        from: file_a.clone(),
                                        to: file_b.clone(),
                                        relationship_type: RelationshipType::SharedDirectory,
                                        evidence: EvidenceSource::Deterministic(format!(
                                            "Files in related directories: {}/ and {}/",
                                            dir_a, dir_b
                                        )),
                                        confidence: 0.6,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    relationships
}

pub fn find_test_files(files: &[PathBuf]) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    for file in files {
        let test_files = find_test_for_file(file, files);
        for test_file in test_files {
            relationships.push(Relationship {
                from: file.clone(),
                to: test_file,
                relationship_type: RelationshipType::TestCoverage,
                evidence: EvidenceSource::Deterministic(format!(
                    "Test file matches naming convention for {:?}",
                    file
                )),
                confidence: 0.9,
            });
        }
    }

    relationships
}

fn find_test_for_file(file: &Path, all_files: &[PathBuf]) -> Vec<PathBuf> {
    let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_parent = file.parent();

    let mut test_files = Vec::new();

    for candidate in all_files {
        let candidate_stem = candidate.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let candidate_parent = candidate.parent();

        let is_test = candidate_stem.ends_with("_test")
            || candidate_stem.ends_with(".test")
            || candidate_stem.ends_with("_spec")
            || candidate_stem.ends_with(".spec");

        if !is_test {
            continue;
        }

        let base_name = candidate_stem
            .trim_end_matches("_test")
            .trim_end_matches(".test")
            .trim_end_matches("_spec")
            .trim_end_matches(".spec");

        if base_name == file_stem {
            test_files.push(candidate.clone());
        }

        if let (Some(fp), Some(cp)) = (file_parent, candidate_parent) {
            if fp == cp {
                let candidate_dir = candidate.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str());
                if candidate_dir == Some("tests") || candidate_dir == Some("test") {
                    if base_name == file_stem {
                        test_files.push(candidate.clone());
                    }
                }
            }
        }
    }

    test_files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_test_files_rust() {
        let files = vec![
            PathBuf::from("src/auth.rs"),
            PathBuf::from("src/auth_test.rs"),
            PathBuf::from("src/user.rs"),
            PathBuf::from("tests/auth.rs"),
        ];

        let relationships = find_test_files(&files);

        assert!(!relationships.is_empty());

        let auth_relationships: Vec<_> = relationships
            .iter()
            .filter(|r| r.from == PathBuf::from("src/auth.rs"))
            .collect();

        assert_eq!(auth_relationships.len(), 2);
    }

    #[test]
    fn test_relationship_graph() {
        let mut graph = RelationshipGraph::new();

        graph.add_relationship(Relationship {
            from: PathBuf::from("src/auth.rs"),
            to: PathBuf::from("src/auth_test.rs"),
            relationship_type: RelationshipType::TestCoverage,
            evidence: EvidenceSource::Deterministic("test".to_string()),
            confidence: 0.9,
        });

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        let related = graph.find_related_files(Path::new("src/auth.rs"));
        assert_eq!(related.len(), 1);
        assert_eq!(related[0], &PathBuf::from("src/auth_test.rs"));
    }

    #[test]
    fn test_find_test_for_file() {
        let files = vec![
            PathBuf::from("src/auth.rs"),
            PathBuf::from("src/auth_test.rs"),
            PathBuf::from("src/user.rs"),
        ];

        let test_files = find_test_for_file(Path::new("src/auth.rs"), &files);

        assert_eq!(test_files.len(), 1);
        assert_eq!(test_files[0], PathBuf::from("src/auth_test.rs"));
    }
}
