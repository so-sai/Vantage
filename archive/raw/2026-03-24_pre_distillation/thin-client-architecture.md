# Thin Client Architecture

> **Thin Client = Intent Emitter, not Processor**

**Version**: 1.0.0  
**Status**: Specification (not implemented)  
**Last Updated**: 2026-01-21

---

## 1. What is a Thin Client?

A Thin Client is a **minimal UI adapter** that:
- Captures user context (selection, file, workspace)
- Builds a `CognitiveIntent` (schema v1)
- Sends to MCP for execution

### 1.1 The Core Contract

```
User Action → Thin Client → CognitiveIntent → MCP → Core → Artifact
```

Every Thin Client, regardless of IDE, follows this exact flow.

### 1.2 Thin Client Responsibilities

| ✅ MUST DO | ❌ MUST NOT DO |
|-----------|----------------|
| Capture user selection | Parse documents |
| Build valid Intent | Analyze content |
| Send to MCP | Execute export logic |
| Display result/error | Cache or transform data |
| Resolve workspace root | Implement business rules |

---

## 2. The Intent Emitter Model

### 2.1 Input: User Context

Every IDE provides some form of:

```
UserContext {
  active_file: Option<Path>,
  selection: Option<Range>,
  workspace_root: Path,
  trigger: Trigger,  // menu, shortcut, command palette
}
```

### 2.2 Output: CognitiveIntent

The Thin Client transforms `UserContext` → `CognitiveIntent`:

```rust
CognitiveIntent {
    intent: IntentKind,      // Lens | Export | Drift
    source: IntentSource,    // VsCode | OpenCode | Antigravity
    target: IntentTarget,    // File { path, selection? } | Workspace { root }
    params: IntentParams,    // { mode: "visual", ... }
}
```

### 2.3 Transformation Rules

| User Action | Intent | Target |
|-------------|--------|--------|
| Right-click file → "Analyze" | `Lens` | `File { path }` |
| Select text → "Export Selection" | `Export` | `File { path, selection }` |
| Context menu → "Export to PDF" | `Export` | `File { path }` |
| Command palette → "Check Drift" | `Drift` | `Workspace { root }` |

---

## 3. What is NOT a Thin Client?

### 3.1 NOT a Document Processor

```
❌ Wrong: Thin Client reads file, extracts headers, passes structured data
✅ Right: Thin Client passes file path, Core reads and processes
```

### 3.2 NOT a Viewer

```
❌ Wrong: Thin Client renders PDF in panel
✅ Right: Thin Client returns PDF path, OS opens with default viewer
```

### 3.3 NOT a Cache Layer

```
❌ Wrong: Thin Client caches DocumentData for faster re-export
✅ Right: Every intent goes to MCP, Core handles optimization
```

### 3.4 NOT a Framework Extension Point

```
❌ Wrong: Users can register custom intent handlers in Thin Client
✅ Right: Thin Client is sealed, new capabilities go in Core
```

---

## 4. Adapter Mapping by IDE

### 4.1 Common Interface

All adapters must implement:

```typescript
interface ThinClient {
  // Build intent from current context
  buildIntent(action: ActionType): CognitiveIntent;
  
  // Send intent to MCP and handle response
  dispatch(intent: CognitiveIntent): Promise<Result>;
  
  // Register UI triggers (menus, commands)
  registerTriggers(): void;
}
```

### 4.2 VS Code Adapter

**Context Mapping:**

| VS Code API | Intent Field |
|-------------|--------------|
| `window.activeTextEditor.document.uri` | `target.path` |
| `editor.selection` | `target.selection` (as ByteSpan) |
| `workspace.workspaceFolders[0]` | `target.root` (for Drift) |

**Selection → ByteSpan:**

```typescript
function selectionToByteSpan(doc: TextDocument, sel: Selection): ByteSpan {
  return {
    start: doc.offsetAt(sel.start),
    end: doc.offsetAt(sel.end)
  };
}
```

**UI Triggers:**

- Context menu: "Vantage: Analyze Document"
- Context menu: "Vantage: Export to PDF"
- Command palette: "Vantage: Check Drift"
- Keyboard shortcut: Ctrl+Shift+V → Export

### 4.3 Open Code Adapter

**Differences from VS Code:**

| Aspect | VS Code | Open Code |
|--------|---------|-----------|
| Workspace API | `workspaceFolders` | `getWorkspaceRoot()` |
| Selection model | `Selection` object | Buffer range |
| Extension format | VSIX | Native plugin |

**Context Mapping:**

```typescript
// Open Code specific
const activeFile = editor.getBufferPath();
const selection = editor.getSelectedBufferRange();
const workspace = project.getRootDirectory();
```

### 4.4 Antigravity Adapter

**Antigravity-specific considerations:**

- Runs inside Google's agent runtime
- MCP already integrated
- May have direct tool access

**Minimal Adapter:**

```typescript
// Antigravity may not need full adapter
// Just UI → Intent → existing MCP channel

function onContextMenuAction(action: string, context: AgentContext) {
  const intent = {
    intent: actionToIntentKind(action),
    source: "antigravity",
    target: contextToTarget(context),
    params: {}
  };
  
  // Use existing MCP connection
  return mcp.callTool(intent.intent, intentToToolParams(intent));
}
```

### 4.5 Claude Code Adapter

**Near-zero adaptation needed:**

- Claude Code has native MCP support
- Agent already understands Intent semantics
- UI triggers may be optional (prompt-based)

**Integration Pattern:**

```
User prompt: "Export this file to PDF"
     ↓
Claude infers: IntentKind::Export
     ↓
Claude builds: CognitiveIntent (from context)
     ↓
Claude calls: MCP tool directly
```

---

## 5. Error Handling

### 5.1 Thin Client Errors (Local)

| Error | Handling |
|-------|----------|
| No file open | Show notification, don't send intent |
| Invalid selection | Send without selection, let Core decide |
| MCP unreachable | Show error, offer retry |

### 5.2 Core Errors (Remote)

| Error | Handling |
|-------|----------|
| `UnsupportedFileType` | Show message with supported types |
| `InvalidDocument` | Show Core's error message |
| `ExportError` | Show message, log for debugging |

### 5.3 Error Display Pattern

```typescript
async function dispatch(intent: CognitiveIntent): Promise<void> {
  try {
    const result = await mcp.call(intent);
    showSuccess(`✅ ${intent.intent} completed`);
  } catch (error) {
    if (error instanceof McpError) {
      showError(`❌ ${error.message}`);
    } else {
      showError(`❌ Unexpected error: ${error}`);
      logError(error); // For debugging
    }
  }
}
```

---

## 6. UI Guidelines

### 6.1 Menu Structure

```
Context Menu (on file):
├── Vantage
│   ├── 📄 Analyze Document     [Lens]
│   ├── 📤 Export to PDF        [Export → Semantic]
│   ├── 📤 Export (Visual)      [Export → Visual]
│   ├── 📤 Export (Audit)       [Export → Audit]
│   └── ─────────────────
│       └── 🔍 Check Drift      [Drift]
```

### 6.2 Status Indicators

| State | Indicator |
|-------|-----------|
| Processing | Spinner in status bar |
| Success | ✅ notification (auto-dismiss) |
| Error | ❌ notification (manual dismiss) |

### 6.3 Output Handling

| Intent | Output Action |
|--------|---------------|
| Lens | Show analysis in output panel |
| Export | Open PDF with system default |
| Drift | Show report in output panel |

---

## 7. Implementation Checklist

### 7.1 Minimal Viable Adapter

- [ ] Register context menu commands
- [ ] Get active file path
- [ ] Get workspace root
- [ ] Build `CognitiveIntent` (Lens, Export, Drift)
- [ ] Send to MCP
- [ ] Display success/error

### 7.2 Enhanced Adapter

- [ ] Selection → ByteSpan conversion
- [ ] Command palette integration
- [ ] Keyboard shortcuts
- [ ] Status bar integration
- [ ] Output panel for reports

### 7.3 Polish

- [ ] Icons for menu items
- [ ] Localization
- [ ] Settings (default export mode)
- [ ] Telemetry (opt-in)

---

## 8. Testing Strategy

### 8.1 Unit Tests (Adapter Logic)

```typescript
describe('Intent Builder', () => {
  it('builds Lens intent from file context', () => {
    const context = { activeFile: '/path/to/file.md' };
    const intent = buildIntent('lens', context);
    
    expect(intent.intent).toBe('lens');
    expect(intent.target.type).toBe('file');
    expect(intent.target.path).toBe('/path/to/file.md');
  });
});
```

### 8.2 Integration Tests (MCP Round-trip)

```typescript
describe('MCP Integration', () => {
  it('successfully calls document_lens', async () => {
    const intent = buildLensIntent('/test/file.md');
    const result = await dispatch(intent);
    
    expect(result.success).toBe(true);
    expect(result.data.headers).toBeDefined();
  });
});
```

---

## 9. Migration Path

### From CLI to Thin Client

CLI is the reference implementation. Thin Client should:

1. **Match CLI behavior exactly**
   - Same intent construction
   - Same error messages
   - Same output format

2. **Add only UI concerns**
   - Menus, triggers
   - Visual feedback
   - File dialogs for output path

3. **Never diverge from CLI semantics**
   - If CLI works, Thin Client works
   - If CLI fails, Thin Client fails the same way

---

## 10. Summary

### The Golden Rule

> **Thin Client builds Intent. Core does everything else.**

### The Implementation Priority

1. **VS Code** - Largest user base, best documentation
2. **Open Code** - Similar to VS Code, quick port
3. **Antigravity** - Already has MCP, minimal work
4. **Claude Code** - May not need explicit adapter

### The Success Metric

A Thin Client is complete when:
- All 3 intent types work (Lens, Export, Drift)
- Output matches CLI exactly
- User can trigger via menu or shortcut
- Errors are displayed clearly

---

*This document specifies the architecture for all IDE Thin Clients. Implementation should follow this specification exactly.*
