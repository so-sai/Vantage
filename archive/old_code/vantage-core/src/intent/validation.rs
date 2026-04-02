//! # Intent Validation Rules
//!
//! This module contains validation logic for CognitiveIntent.
//!
//! ## Validation Philosophy
//! - Validate **semantics**, not UI/UX
//! - Validate **structural correctness**, not business rules
//! - Fail fast with clear error messages
//!
//! ## Rules (v1)
//! 1. `Lens` requires `File` target
//! 2. `Export` requires `File` target  
//! 3. `Drift` requires `Workspace` target
//! 4. File paths must not be empty
//! 5. Workspace roots must not be empty
//! 6. ByteSpan must have start < end (if present)

use super::{CognitiveIntent, IntentKind, IntentTarget, IntentValidationError};

/// Validate a CognitiveIntent according to schema rules.
///
/// # Returns
/// - `Ok(())` if the intent is valid
/// - `Err(IntentValidationError)` if validation fails
///
/// # Example
/// ```rust
/// use vantage_core::intent::{CognitiveIntent, IntentKind, IntentTarget, IntentSource, validate_intent};
/// use std::path::PathBuf;
///
/// let intent = CognitiveIntent {
///     intent: IntentKind::Lens,
///     source: IntentSource::Cli,
///     target: IntentTarget::File {
///         path: PathBuf::from("test.md"),
///         selection: None,
///     },
///     params: Default::default(),
/// };
///
/// assert!(validate_intent(&intent).is_ok());
/// ```
pub fn validate_intent(intent: &CognitiveIntent) -> Result<(), IntentValidationError> {
    // Rule 1-3: Intent-Target compatibility
    validate_intent_target_compatibility(intent)?;

    // Rule 4-5: Path validation
    validate_target_paths(&intent.target)?;

    // Rule 6: ByteSpan validation (if present)
    validate_byte_span(&intent.target)?;

    Ok(())
}

/// Validate that the intent kind is compatible with the target type.
fn validate_intent_target_compatibility(intent: &CognitiveIntent) -> Result<(), IntentValidationError> {
    match (&intent.intent, &intent.target) {
        // Lens requires File
        (IntentKind::Lens, IntentTarget::Workspace { .. }) => {
            Err(IntentValidationError::LensRequiresFile)
        }
        // Export requires File
        (IntentKind::Export, IntentTarget::Workspace { .. }) => {
            Err(IntentValidationError::ExportRequiresFile)
        }
        // Drift requires Workspace
        (IntentKind::Drift, IntentTarget::File { .. }) => {
            Err(IntentValidationError::DriftRequiresWorkspace)
        }
        // All other combinations are valid
        _ => Ok(()),
    }
}

/// Validate that paths are not empty.
fn validate_target_paths(target: &IntentTarget) -> Result<(), IntentValidationError> {
    match target {
        IntentTarget::File { path, .. } => {
            if path.as_os_str().is_empty() {
                return Err(IntentValidationError::EmptyFilePath);
            }
        }
        IntentTarget::Workspace { root } => {
            if root.as_os_str().is_empty() {
                return Err(IntentValidationError::EmptyWorkspaceRoot);
            }
        }
    }
    Ok(())
}

/// Validate ByteSpan if present.
fn validate_byte_span(target: &IntentTarget) -> Result<(), IntentValidationError> {
    if let IntentTarget::File { selection: Some(span), .. } = target
        && span.start >= span.end
    {
        return Err(IntentValidationError::InvalidByteSpan {
            start: span.start,
            end: span.end,
        });
    }
    Ok(())
}


// ============================================================
// TDD TESTS - Lock Intent Schema v1 Behavior
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{ByteSpan, IntentBuilder, IntentSource};
    use std::path::PathBuf;

    // ----------------------------------------------------------
    // TEST 1: Valid Lens intent with File target
    // ----------------------------------------------------------
    #[test]
    fn test_valid_lens_with_file() {
        let intent = CognitiveIntent {
            intent: IntentKind::Lens,
            source: IntentSource::Cli,
            target: IntentTarget::File {
                path: PathBuf::from("README.md"),
                selection: None,
            },
            params: Default::default(),
        };

        assert!(validate_intent(&intent).is_ok());
    }

    // ----------------------------------------------------------
    // TEST 2: Valid Export intent with File target
    // ----------------------------------------------------------
    #[test]
    fn test_valid_export_with_file() {
        let intent = CognitiveIntent {
            intent: IntentKind::Export,
            source: IntentSource::VsCode,
            target: IntentTarget::file("design.md"),
            params: Default::default(),
        };

        assert!(validate_intent(&intent).is_ok());
    }

    // ----------------------------------------------------------
    // TEST 3: Valid Drift intent with Workspace target
    // ----------------------------------------------------------
    #[test]
    fn test_valid_drift_with_workspace() {
        let intent = CognitiveIntent {
            intent: IntentKind::Drift,
            source: IntentSource::Antigravity,
            target: IntentTarget::workspace("./project"),
            params: Default::default(),
        };

        assert!(validate_intent(&intent).is_ok());
    }

    // ----------------------------------------------------------
    // TEST 4: Lens with Workspace target → Error
    // ----------------------------------------------------------
    #[test]
    fn test_lens_requires_file_not_workspace() {
        let intent = CognitiveIntent {
            intent: IntentKind::Lens,
            source: IntentSource::Cli,
            target: IntentTarget::workspace("./project"),
            params: Default::default(),
        };

        let result = validate_intent(&intent);
        assert_eq!(result, Err(IntentValidationError::LensRequiresFile));
    }

    // ----------------------------------------------------------
    // TEST 5: Export with Workspace target → Error
    // ----------------------------------------------------------
    #[test]
    fn test_export_requires_file_not_workspace() {
        let intent = CognitiveIntent {
            intent: IntentKind::Export,
            source: IntentSource::ClaudeCode,
            target: IntentTarget::workspace("./docs"),
            params: Default::default(),
        };

        let result = validate_intent(&intent);
        assert_eq!(result, Err(IntentValidationError::ExportRequiresFile));
    }

    // ----------------------------------------------------------
    // TEST 6: Drift with File target → Error
    // ----------------------------------------------------------
    #[test]
    fn test_drift_requires_workspace_not_file() {
        let intent = CognitiveIntent {
            intent: IntentKind::Drift,
            source: IntentSource::OpenCode,
            target: IntentTarget::file("single_file.rs"),
            params: Default::default(),
        };

        let result = validate_intent(&intent);
        assert_eq!(result, Err(IntentValidationError::DriftRequiresWorkspace));
    }

    // ----------------------------------------------------------
    // TEST 7: Empty file path → Error
    // ----------------------------------------------------------
    #[test]
    fn test_empty_file_path_rejected() {
        let intent = CognitiveIntent {
            intent: IntentKind::Lens,
            source: IntentSource::Cli,
            target: IntentTarget::File {
                path: PathBuf::from(""),
                selection: None,
            },
            params: Default::default(),
        };

        let result = validate_intent(&intent);
        assert_eq!(result, Err(IntentValidationError::EmptyFilePath));
    }

    // ----------------------------------------------------------
    // TEST 8: Empty workspace root → Error  
    // ----------------------------------------------------------
    #[test]
    fn test_empty_workspace_root_rejected() {
        let intent = CognitiveIntent {
            intent: IntentKind::Drift,
            source: IntentSource::Cli,
            target: IntentTarget::Workspace {
                root: PathBuf::from(""),
            },
            params: Default::default(),
        };

        let result = validate_intent(&intent);
        assert_eq!(result, Err(IntentValidationError::EmptyWorkspaceRoot));
    }

    // ----------------------------------------------------------
    // TEST 9: Invalid ByteSpan (start >= end) → Error
    // ----------------------------------------------------------
    #[test]
    fn test_invalid_byte_span_rejected() {
        let intent = CognitiveIntent {
            intent: IntentKind::Lens,
            source: IntentSource::Cli,
            target: IntentTarget::File {
                path: PathBuf::from("test.md"),
                selection: Some(ByteSpan { start: 100, end: 50 }),
            },
            params: Default::default(),
        };

        let result = validate_intent(&intent);
        assert_eq!(
            result,
            Err(IntentValidationError::InvalidByteSpan { start: 100, end: 50 })
        );
    }

    // ----------------------------------------------------------
    // TEST 10: Valid ByteSpan passes
    // ----------------------------------------------------------
    #[test]
    fn test_valid_byte_span_accepted() {
        let intent = CognitiveIntent {
            intent: IntentKind::Lens,
            source: IntentSource::Cli,
            target: IntentTarget::file_with_selection("test.md", 0, 100),
            params: Default::default(),
        };

        assert!(validate_intent(&intent).is_ok());
    }

    // ----------------------------------------------------------
    // TEST 11: IntentBuilder produces valid intent
    // ----------------------------------------------------------
    #[test]
    fn test_builder_creates_valid_intent() {
        let result = IntentBuilder::new()
            .kind(IntentKind::Export)
            .source(IntentSource::VsCode)
            .target(IntentTarget::file("output.md"))
            .param("mode", "visual")
            .build();

        assert!(result.is_ok());
        let intent = result.expect("should be valid");
        assert_eq!(intent.intent, IntentKind::Export);
        assert_eq!(intent.source, IntentSource::VsCode);
    }

    // ----------------------------------------------------------
    // TEST 12: IntentBuilder fails on missing kind
    // ----------------------------------------------------------
    #[test]
    fn test_builder_fails_without_kind() {
        let result = IntentBuilder::new()
            .source(IntentSource::Cli)
            .target(IntentTarget::file("test.md"))
            .build();

        assert_eq!(result, Err(IntentValidationError::MissingIntentKind));
    }

    // ----------------------------------------------------------
    // TEST 13: IntentBuilder fails on missing target
    // ----------------------------------------------------------
    #[test]
    fn test_builder_fails_without_target() {
        let result = IntentBuilder::new()
            .kind(IntentKind::Lens)
            .source(IntentSource::Cli)
            .build();

        assert_eq!(result, Err(IntentValidationError::MissingTarget));
    }

    // ----------------------------------------------------------
    // TEST 14: MCP tool name mapping is correct
    // ----------------------------------------------------------
    #[test]
    fn test_mcp_tool_name_mapping() {
        assert_eq!(IntentKind::Lens.mcp_tool_name(), "document_lens");
        assert_eq!(IntentKind::Export.mcp_tool_name(), "export_to_pdf");
        assert_eq!(IntentKind::Drift.mcp_tool_name(), "check_drift");
    }

    // ----------------------------------------------------------
    // TEST 15: ByteSpan utility methods
    // ----------------------------------------------------------
    #[test]
    fn test_byte_span_utilities() {
        let span = ByteSpan::new(10, 50);
        assert_eq!(span.len(), 40);
        assert!(!span.is_empty());

        let empty_span = ByteSpan::new(50, 50);
        assert!(empty_span.is_empty());
        assert_eq!(empty_span.len(), 0);
    }
}
