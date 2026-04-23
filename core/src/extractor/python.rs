use streaming_iterator::StreamingIterator;
use vantage_types::{EdgeEvent, EdgeType};
use tree_sitter::{Parser, Query, QueryCursor};

pub fn extract_imports(source_code: &str, file_path: &str) -> Vec<EdgeEvent> {
    let mut parser = Parser::new();
    let lang = tree_sitter_python::LANGUAGE.into();
    parser.set_language(&lang).expect("Error loading Python grammar");

    let tree = parser.parse(source_code, None).expect("Error parsing code");
    let root_node = tree.root_node();

    let query_str = r#"
        (import_statement
            name: (dotted_name) @import_name)
        (import_from_statement
            module_name: [(dotted_name) (relative_import)] @from_module)
        (class_definition
            name: (identifier) @class_name
            superclasses: (argument_list [
                (identifier) 
                (attribute)
            ] @base_name)?)
    "#;

    let query = Query::new(&lang, query_str).expect("Error creating query");
    let mut cursor = QueryCursor::new();

    let mut edges = Vec::new();
    let mut current_class: Option<String> = None;

    // Use while let pattern for tree-sitter 0.24
    let mut matches = cursor.matches(&query, root_node, source_code.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let node = capture.node;
            let capture_name = query.capture_names()[capture.index as usize];
            let text = node.utf8_text(source_code.as_bytes()).unwrap_or_default().to_string();
            let line = node.start_position().row + 1;
            let raw_text = text.clone();

            let source_module = file_path.replace('\\', "/").replace(".py", "").replace('/', ".");

            match capture_name {
                "import_name" | "from_module" => {
                    edges.push(EdgeEvent {
                        source: source_module.clone(),
                        target: text,
                        edge_type: EdgeType::Imports,
                        language: "python".to_string(),
                        source_file: file_path.replace('\\', "/"),
                        target_file: None,
                        line,
                        confidence: 1.0,
                        raw_text,
                    });
                }
                "class_name" => {
                    current_class = Some(text);
                }
                "base_name" => {
                    if let Some(ref cls) = current_class {
                        edges.push(EdgeEvent {
                            source: cls.clone(),
                            target: text,
                            edge_type: EdgeType::Inherits,
                            language: "python".to_string(),
                            source_file: file_path.replace('\\', "/"),
                            target_file: None,
                            line,
                            confidence: 1.0,
                            raw_text,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_import() {
        let code = r#"import os"#;
        let edges = extract_imports(code, "test.py");
        assert!(edges.iter().any(|e| e.target == "os" && matches!(e.edge_type, EdgeType::Imports)));
    }

    #[test]
    fn test_extract_from_import() {
        let code = r#"from os import path"#;
        let edges = extract_imports(code, "test.py");
        assert!(edges.iter().any(|e| e.target == "os" && matches!(e.edge_type, EdgeType::Imports)));
    }

    #[test]
    fn test_extract_relative_import() {
        let code = r#"from . import utils"#;
        let edges = extract_imports(code, "kit/api.py");
        assert!(edges.iter().any(|e| e.raw_text == "."));
    }

    #[test]
    fn test_extract_inheritance() {
        let code = r#"
class Base:
    pass

class Sub(Base):
    pass
"#;
        let edges = extract_imports(code, "test.py");
        assert!(edges.iter().any(|e| 
            e.source == "Sub" && 
            e.target == "Base" && 
            matches!(e.edge_type, EdgeType::Inherits)
        ));
    }
}