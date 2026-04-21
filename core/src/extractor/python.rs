use vantage_types::EdgeEvent;
use tree_sitter::{Parser, Query, QueryCursor};
use streaming_iterator::StreamingIterator;

pub fn extract_imports(source_code: &str, file_path: &str) -> Vec<EdgeEvent> {
    let mut parser = Parser::new();
    let lang = tree_sitter_python::LANGUAGE.into();
    parser
        .set_language(&lang)
        .expect("Error loading Python grammar");

    let tree = parser.parse(source_code, None).expect("Error parsing code");
    let root_node = tree.root_node();

    // Query to find imports and class inheritance
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
    let mut matches = cursor.matches(&query, root_node, source_code.as_bytes());

    let mut edges = Vec::new();

    let mut current_class: Option<String> = None;

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let node = capture.node;
            let capture_name = query.capture_names()[capture.index as usize];
            let text = node.utf8_text(source_code.as_bytes()).unwrap_or_default().to_string();
            let line = node.start_position().row + 1;
            let raw_text = text.clone();

            let source_module = file_path.replace("\\", "/").replace(".py", "").replace("/", ".");

            match capture_name {
                "import_name" | "from_module" => {
                    edges.push(EdgeEvent {
                        source: source_module.clone(),
                        target: text,
                        edge_type: "IMPORTS".to_string(),
                        language: "python".to_string(),
                        source_file: file_path.replace("\\", "/"),
                        line,
                        confidence: 1.0,
                        raw_text,
                        schema_version: 1,
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
                            edge_type: "INHERITS".to_string(),
                            language: "python".to_string(),
                            source_file: file_path.replace("\\", "/"),
                            line,
                            confidence: 1.0,
                            raw_text,
                            schema_version: 1,
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
    fn test_extract_imports() {
        let code = r#"
import os
import sys, time
from os import path
from ..core import auth
"#;
        let edges = extract_imports(code, "test.py");
        
        // Expected: os, sys, time, os, ..core
        assert!(edges.iter().any(|e| e.target == "os"));
        assert!(edges.iter().any(|e| e.target == "sys"));
        assert!(edges.iter().any(|e| e.target == "time"));
        assert!(edges.iter().any(|e| e.target == "..core"));
    }

    #[test]
    fn test_extract_inheritance() {
        let code = r#"
class Base:
    pass

class Sub(Base):
    pass

class Complex(core.Base, mixins.Auth):
    pass
"#;
        let edges = extract_imports(code, "test.py");
        
        // Expected: Sub -> Base, Complex -> core.Base, Complex -> mixins.Auth
        assert!(edges.iter().any(|e| e.source == "Sub" && e.target == "Base" && e.edge_type == "INHERITS"));
        assert!(edges.iter().any(|e| e.source == "Complex" && e.target == "core.Base" && e.edge_type == "INHERITS"));
        assert!(edges.iter().any(|e| e.source == "Complex" && e.target == "mixins.Auth" && e.edge_type == "INHERITS"));
    }
}
