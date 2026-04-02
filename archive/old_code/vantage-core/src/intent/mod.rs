//! # Cognitive Intent Schema v1
//!
//! This module defines the core intent protocol layer between:
//! - **Users** (via IDE or CLI)
//! - **Adapters** (VS Code, Open Code, Antigravity, CLI)
//! - **MCP** (tool dispatch)
//! - **Vantage Core** (execution)
//!
//! ## Design Principles
//! 1. **Semantic, not UI** - No editor/cursor/panel concepts
//! 2. **Stateless, not session** - Each intent is self-contained
//! 3. **1:1 MCP mapping** - Every intent maps to exactly one MCP tool call
//!
//! ## Example
//! ```rust
//! use vantage_core::intent::{CognitiveIntent, IntentKind, IntentTarget, IntentSource};
//! use std::path::PathBuf;
//!
//! let intent = CognitiveIntent {
//!     intent: IntentKind::Lens,
//!     source: IntentSource::Cli,
//!     target: IntentTarget::File {
//!         path: PathBuf::from("README.md"),
//!         selection: None,
//!     },
//!     params: Default::default(),
//! };
//! ```

mod validation;

pub use validation::validate_intent;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Core intent structure - the protocol between adapters and MCP.
///
/// This is intentionally minimal. Adapters emit this; MCP consumes this.
/// No UI concepts leak through.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveIntent {
    /// WHAT action to perform
    pub intent: IntentKind,
    /// WHERE this intent originated (telemetry, not logic)
    pub source: IntentSource,
    /// WHAT the action operates on
    pub target: IntentTarget,
    /// HOW to perform (opaque, pass-through to MCP)
    pub params: IntentParams,
}

/// The type of cognitive action to perform.
///
/// This enum is intentionally closed. New intents require schema version bump.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    /// Analyze document structure → `document_lens` tool
    Lens,
    /// Export to PDF → `export_to_pdf` tool
    Export,
    /// Check code-doc drift → `check_drift` tool
    Drift,
}

impl IntentKind {
    /// Returns the MCP tool name this intent maps to
    pub fn mcp_tool_name(&self) -> &'static str {
        match self {
            IntentKind::Lens => "document_lens",
            IntentKind::Export => "export_to_pdf",
            IntentKind::Drift => "check_drift",
        }
    }
}

/// Origin of the intent (for telemetry and debugging, NOT logic).
///
/// Adapters should set this honestly. Core does not branch on source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    /// Command-line interface adapter
    #[default]
    Cli,
    /// VS Code extension
    VsCode,
    /// Open Code IDE
    OpenCode,
    /// Antigravity (Google)
    Antigravity,
    /// Claude Code
    ClaudeCode,
    /// Direct MCP call (no adapter)
    DirectMcp,
    /// Unknown/other source
    Unknown,
}

/// The target of a cognitive action.
///
/// This is semantic: file or workspace. No UI concepts (cursor, selection in editor sense).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IntentTarget {
    /// Target is a specific file
    File {
        /// Absolute or relative path to the file
        path: PathBuf,
        /// Optional byte-range selection within the file
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selection: Option<ByteSpan>,
    },
    /// Target is a workspace/directory
    Workspace {
        /// Root path of the workspace
        root: PathBuf,
    },
}

impl IntentTarget {
    /// Helper to create a File target
    pub fn file(path: impl Into<PathBuf>) -> Self {
        IntentTarget::File {
            path: path.into(),
            selection: None,
        }
    }

    /// Helper to create a File target with selection
    pub fn file_with_selection(path: impl Into<PathBuf>, start: usize, end: usize) -> Self {
        IntentTarget::File {
            path: path.into(),
            selection: Some(ByteSpan { start, end }),
        }
    }

    /// Helper to create a Workspace target
    pub fn workspace(root: impl Into<PathBuf>) -> Self {
        IntentTarget::Workspace { root: root.into() }
    }

    /// Get the primary path from this target
    pub fn path(&self) -> &PathBuf {
        match self {
            IntentTarget::File { path, .. } => path,
            IntentTarget::Workspace { root } => root,
        }
    }
}

/// Byte range within a file (not line-based, portable across editors).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByteSpan {
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
}

impl ByteSpan {
    /// Create a new ByteSpan
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Opaque parameters passed through to MCP tools.
///
/// Adapters can put anything here. MCP tools extract what they need.
/// This allows forward-compatibility without schema changes.
pub type IntentParams = HashMap<String, serde_json::Value>;

/// Builder for constructing CognitiveIntent instances.
///
/// Provides a fluent API for creating intents with validation.
#[derive(Debug, Default)]
pub struct IntentBuilder {
    intent: Option<IntentKind>,
    source: IntentSource,
    target: Option<IntentTarget>,
    params: IntentParams,
}

impl IntentBuilder {
    /// Create a new IntentBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the intent kind
    pub fn kind(mut self, kind: IntentKind) -> Self {
        self.intent = Some(kind);
        self
    }

    /// Set the source
    pub fn source(mut self, source: IntentSource) -> Self {
        self.source = source;
        self
    }

    /// Set the target
    pub fn target(mut self, target: IntentTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Add a parameter
    pub fn param(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Build and validate the intent
    pub fn build(self) -> Result<CognitiveIntent, IntentValidationError> {
        let intent = CognitiveIntent {
            intent: self.intent.ok_or(IntentValidationError::MissingIntentKind)?,
            source: self.source,
            target: self.target.ok_or(IntentValidationError::MissingTarget)?,
            params: self.params,
        };
        validate_intent(&intent)?;
        Ok(intent)
    }
}

/// Errors that can occur during intent validation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IntentValidationError {
    #[error("Missing intent kind - must specify Lens, Export, or Drift")]
    MissingIntentKind,

    #[error("Missing target - must specify File or Workspace")]
    MissingTarget,

    #[error("Lens intent requires File target, got Workspace")]
    LensRequiresFile,

    #[error("Export intent requires File target, got Workspace")]
    ExportRequiresFile,

    #[error("Drift intent requires Workspace target, got File")]
    DriftRequiresWorkspace,

    #[error("File path is empty")]
    EmptyFilePath,

    #[error("Workspace root is empty")]
    EmptyWorkspaceRoot,

    #[error("Invalid byte span: start ({start}) >= end ({end})")]
    InvalidByteSpan { start: usize, end: usize },
}
