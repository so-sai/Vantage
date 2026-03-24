# Vantage Supported Format Policy

**Purpose**: Lock scope, enforce semantic contracts, prevent viewer zoo.

---

## Core Principle

Each supported format MUST:
- Map deterministically to `DocumentData`
- Preserve semantic hierarchy (headers, structure, intent)
- Be exportable to PDF without LLM (zero-token primitive)

---

## Tier A – Zero-Token Primitives (Ship First)

### Technical Knowledge Artifacts
- `.md` (Markdown) – **Canonical format**, reference implementation
- `.rst` (reStructuredText) – Python/Sphinx documentation
- `.adoc` (AsciiDoc) – Long-form technical specs

**Cognitive Role**: Design documents, specifications, architecture decisions

**Required Outputs**:
- `headers: Vec<String>` – Semantic tree
- `text: String` – Full content
- `metadata: Metadata` – Word count, last modified

---

### System Descriptors
- `.yaml`, `.yml` – Configuration, API specs
- `.json` – Structured data, schemas
- `.toml` – Rust project manifests

**Cognitive Role**: System boundary definition, contract enforcement

**Required Outputs**:
- Key-value hierarchy
- Schema validation (future)
- Machine-readable appendix for PDF

---

## Tier B – Actionable Intelligence (Phase 2)

### Data-as-Spec
- `.csv`, `.xlsx` – Schema definitions, test data

**Cognitive Role**: Invariant detection, spec vs implementation audit

### Test Artifacts
- `.http` – API test cases
- `.sql` – Database schema/queries
- `.graphql` – API contracts

**Cognitive Role**: Test-as-spec, living documentation

---

## Tier C – Optional (Phase 3+)

### Code Structure (API extraction only)
- `.rs`, `.py`, `.ts`, `.go`

**Cognitive Role**: Public API extraction for drift detection

**NOT supported**:
- Syntax highlighting
- Full AST parsing
- IDE replacement features

---

## Explicitly NOT Supported

| Format | Reason |
|--------|--------|
| `.pdf` | Output artifact, not input |
| `.zip`, `.rar` | Container, no semantic value |
| `.svg`, `.png` | Visual, minimal intent |
| Font files | Not cognitive layer |

---

## Test Contract (Enforcement)

Every format implementation MUST pass:

```rust
#[test]
fn format_to_document_data() {
    let input = load_sample();
    let doc = document_lens(&input)?;
    assert!(!doc.headers.is_empty());
    assert!(doc.metadata.word_count > 0);
}

#[test]
fn format_to_pdf_preserves_semantic() {
    let doc = document_lens(&input)?;
    let pdf = export_to_pdf(&doc, ExportConfig::default())?;
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf_contains_headers(&pdf, &doc.headers));
}
```

---

**Anti-Scope-Creep Rule**:
If a format doesn't fit Tier A/B/C criteria, reject by default.
"Viewing capability" is NOT a valid reason to add support.
