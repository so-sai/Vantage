# Spec: Python AST Mapping (v1.0)
Timestamp: 2026-03-24

## 1. Goal
Map Python 3.14.3 AST nodes to the `CognitiveSignal` schema using a 2-phase protocol.

## 2. Phase-Based Pipeline
1. **Phase A - Extract (AST -> Nodes):**
   - Identify `function_definition`, `class_definition`, `decorated_definition`.
   - Capture `identifier`, `parameters`, `body`.
2. **Phase B - Normalize (Nodes -> CognitiveSignal):**
   - Map `function_definition` -> `SymbolKind::Function`.
   - Map `class_definition` -> `SymbolKind::Class`.
   - Generate `symbol_id` using module + name hierarchy.

## 3. Expected Mappings
| Python Node | SymbolKind | Confidence |
| --- | --- | --- |
| `function_definition` | `function` | 1.0 |
| `class_definition` | `class` | 1.0 |
| `async_function_definition` | `function` | 1.0 |

## 4. Cross-Language Equivalents
- **Rust `function_item`** ↔ **Python `function_definition`**
- **Rust `struct_item`** ↔ **Python `class_definition`** (where applicable)
