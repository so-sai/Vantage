//! # Intent Overlay Extractor — v1.2.5
//!
//! Parses structured `vantage:` comment blocks from source code.
//! Language-agnostic: works with `//`, `#`, `/*` comment styles.
//!
//! ## Supported syntax
//! ```ignore
//! // vantage:
//! //   invariant: AppendOnly
//! //   reason: Prevent rollback corruption
//! //   owner: security-team
//! ```

use std::collections::HashMap;
use vantage_types::intent::{Constraint, ConstraintKind, IntentInvariant, IntentOverlay};

/// Parsed intent block with the end line of the comment block.
#[derive(Debug, Clone)]
pub struct IntentBlock {
    pub overlay: IntentOverlay,
    /// The line number where the intent comment ends (the line AFTER the comment block).
    /// Used to match the intent to the next symbol declaration.
    pub end_line: usize,
}

fn parse_invariant(s: &str) -> Option<IntentInvariant> {
    let trimmed = s.trim();
    match trimmed.to_lowercase().as_str() {
        "appendonly" | "append_only" => Some(IntentInvariant::AppendOnly),
        "deterministicordering" | "deterministic_ordering" => Some(IntentInvariant::DeterministicOrdering),
        "threadsafe" | "thread_safe" => Some(IntentInvariant::ThreadSafe),
        "nosideeffects" | "no_side_effects" => Some(IntentInvariant::NoSideEffects),
        "idempotent" => Some(IntentInvariant::Idempotent),
        "zeroallocation" | "zero_allocation" => Some(IntentInvariant::ZeroAllocation),
        "stateless" => Some(IntentInvariant::Stateless),
        _ => Some(IntentInvariant::Custom(trimmed.to_string())),
    }
}

fn parse_constraint_kind(s: &str) -> Option<ConstraintKind> {
    let s = s.trim();
    if let Some(target) = s.strip_prefix("must_import:") {
        Some(ConstraintKind::MustImport(target.trim().to_string()))
    } else if let Some(target) = s.strip_prefix("must_not_import:") {
        Some(ConstraintKind::MustNotImport(target.trim().to_string()))
    } else if let Some(target) = s.strip_prefix("must_extend:") {
        Some(ConstraintKind::MustExtend(target.trim().to_string()))
    } else if s.trim() == "must_be_sealed" {
        Some(ConstraintKind::MustBeSealed)
    } else if s.trim() == "must_be_stateless" {
        Some(ConstraintKind::MustBeStateless)
    } else {
        None
    }
}

/// Extract all vantage intent blocks from source code.
///
/// Returns a list of intent blocks ordered by their position in the file.
pub fn extract_intents(source: &str) -> Vec<IntentBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Detect start of a vantage intent block
        // Matches: `vantage:` after stripping comment markers and whitespace
        let stripped = strip_comment(line);
        if stripped.trim().starts_with("vantage:")
            || stripped.trim().starts_with("@vantage")
        {
            // Parse the block: read subsequent comment lines with indented keys
            if let Some(overlay) = parse_intent_block(&lines, i) {
                // Determine end_line: the last line of the comment block
                let mut end = i;
                while end + 1 < lines.len() {
                    let next = strip_comment(lines[end + 1]);
                    if next.trim().is_empty() || !lines[end + 1].trim().starts_with(&['#', '/', '*']) {
                        // Check if the intent comment continues
                        let cur = strip_comment(lines[end + 1]);
                        if cur.trim().starts_with(|c: char| c.is_whitespace())
                            && cur.trim().contains(':')
                        {
                            end += 1;
                            continue;
                        }
                        break;
                    }
                    end += 1;
                }
                blocks.push(IntentBlock {
                    end_line: end + 2, // +1 for 1-indexed, +1 for the comment end
                    overlay,
                });
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    blocks
}

/// Strip comment markers from a line.
fn strip_comment(line: &str) -> &str {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("//") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('#') {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("/*") {
        rest.trim_end_matches("*/").trim_end_matches('/')
    } else {
        trimmed
    }
}

/// Parse a vantage intent block starting at the given line index.
fn parse_intent_block(lines: &[&str], start: usize) -> Option<IntentOverlay> {
    let first = strip_comment(lines[start]);
    // First line: `vantage:` or `@vantage` — move past the key
    if !first.contains(':') && !first.contains("@vantage") {
        return None;
    }

    let mut invariant: Option<IntentInvariant> = None;
    let mut reason: Option<String> = None;
    let mut owner: Option<String> = None;
    let mut constraints: Vec<Constraint> = Vec::new();
    let mut metadata: HashMap<String, String> = HashMap::new();

    let mut i = start + 1;
    while i < lines.len() {
        let line = strip_comment(lines[i]);
        let line = line.trim();

        if line.is_empty() {
            i += 1;
            continue;
        }

        // Stop if we hit a non-indented non-comment line
        if !lines[i].trim().starts_with(&['#', '/', '*'])
            && !line.starts_with(|c: char| c.is_whitespace() || line.contains(':'))
        {
            break;
        }

        // Parse key: value pairs
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            match key.as_str() {
                "invariant" => {
                    invariant = parse_invariant(value);
                }
                "reason" => {
                    reason = Some(value.to_string());
                }
                "owner" => {
                    owner = Some(value.to_string());
                }
                k if k.starts_with("must_") => {
                    if let Some(ck) = parse_constraint_kind(&format!("{}:{}", k, value)) {
                        constraints.push(Constraint {
                            kind: ck,
                            description: value.to_string(),
                        });
                    }
                }
                _ => {
                    metadata.insert(key, value.to_string());
                }
            }
        }

        i += 1;
    }

    // At minimum, invariant must be specified
    let invariant = invariant?;

    Some(IntentOverlay {
        invariant,
        reason: reason.unwrap_or_default(),
        constraints,
        owner,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_intent_block() {
        let source = r#"
// vantage:
//   invariant: AppendOnly
//   reason: Prevent rollback corruption
struct EventStore {
    events: Vec<Event>,
}
"#;
        let blocks = extract_intents(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].overlay.invariant, IntentInvariant::AppendOnly);
        assert_eq!(blocks[0].overlay.reason, "Prevent rollback corruption");
    }

    #[test]
    fn test_python_intent_block() {
        let source = r#"
# vantage:
#   invariant: Stateless
#   reason: Safe retry semantics
def handler(event):
    pass
"#;
        let blocks = extract_intents(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].overlay.invariant, IntentInvariant::Stateless);
    }

    #[test]
    fn test_no_intent_block() {
        let source = r#"
fn normal_function() {
    let x = 1;
}
"#;
        let blocks = extract_intents(source);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_multiple_intent_blocks() {
        let source = r#"
// vantage:
//   invariant: Idempotent
//   reason: Safe to retry
fn process() {}

// vantage:
//   invariant: ZeroAllocation
//   reason: Hot path
fn fast() {}
"#;
        let blocks = extract_intents(source);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].overlay.invariant, IntentInvariant::Idempotent);
        assert_eq!(blocks[1].overlay.invariant, IntentInvariant::ZeroAllocation);
    }

    #[test]
    fn test_custom_invariant() {
        let source = r#"
# vantage:
#   invariant: CustomBusinessRule
#   reason: Must pass audit
class AuditLog:
    pass
"#;
        let blocks = extract_intents(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].overlay.invariant,
            IntentInvariant::Custom("CustomBusinessRule".to_string())
        );
    }

    #[test]
    fn test_intent_with_owner() {
        let source = r#"
// vantage:
//   invariant: ThreadSafe
//   reason: Shared state
//   owner: platform-team
struct SharedState {
    counter: AtomicU64,
}
"#;
        let blocks = extract_intents(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].overlay.owner,
            Some("platform-team".to_string())
        );
    }

    #[test]
    fn test_js_comment_style() {
        let source = r#"
/* vantage:
   invariant: Stateless
   reason: Pure component */
function Button() {}
"#;
        let blocks = extract_intents(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].overlay.invariant, IntentInvariant::Stateless);
    }
}
