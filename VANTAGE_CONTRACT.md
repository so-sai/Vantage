# VANTAGE MASTER CONTRACT (v1.2.4)

> **CANONICAL AUTHORITY** for all structural sensors, agents, and CI interactions.
> Vantage is a **stateless structural sensor** providing deterministic code identity.

---

## 1. PURPOSE & IDENTITY
Vantage maps physical source code (L0) to structural symbols (L2) using a triple-hash identity system. It does not interpret intent; it only verifies structure and relationships.

---

## 2. CLI INTERFACE
Vantage is invoked as a single-shot binary with no runtime dependencies.

```bash
vantage <command> [args] [--json]
```

### Commands:
- `verify <file> [--enforce (EXP)]`: Parse source, extract signals, run pipeline.
- `graph <file> (EXP)`: Extract dependency edges (calls, imports).
- `diff <file> [--seal .] (EXP)`: Compare against `VANTAGE.SEAL` baseline.
- `seal <path> (EXP)`: Create forensic baseline for a directory.
- `purge --force (EXP)`: Remove local forensic artifacts.

> [!CAUTION]
> **EXPERIMENTAL MODE ACTIVE (v1.2.4-EXPERIMENTAL)**
> Modules marked with `(EXP)` are operational but unverified on large-scale repositories. AI Agents must treat outputs as forensic evidence requiring high-level reasoning.

---

## 3. OUTPUT MODEL (JSON)
Every JSON output root contains `"v": "1.2.4"` and a `"status"` field.

### CognitiveSignal Schema:
| Field | Type | Description |
| :--- | :--- | :--- |
| `id` | `uuid` | Stable Epistemic identifier from `@epistemic:<uuid>` |
| `name` | `string` | Symbol name (e.g., `calculate_total`) |
| `kind` | `enum` | `Function`, `Class`, `Struct`, `Trait`, `Module`, etc. |
| `structural_hash` | `sha256` | Byte-level identity (detects any change) |
| `semantic_hash` | `sha256` | Whitespace-invariant identity |
| `normalized_hash` | `sha256` | Rename-invariant identity (AST S-expression) |
| `location` | `object` | `{ file, start_line, start_col, end_line, end_col }` |

### FailureReason Taxonomy:
If `status` is `error`, `reason` will be one of:
- `syntax_error`: Source cannot be parsed.
- `unsupported_language`: No grammar for file extension.
- `no_anchor_found`: No `@epistemic` tag detected.
- `file_read_error`: I/O failure.
- `internal_error`: Software failure.

---

## 4. SYSTEM INVARIANTS (NON-NEGOTIABLE)

1. **Determinism**: Same source → identical JSON output across all OS/environments.
2. **Stateless**: No internal memory, no daemon, no background process.
3. **Triple-Hash Matrix**: 
   - Whitespace change → `structural` changes, others same.
   - Variable rename → `structural`/`semantic` change, `normalized` same.
   - Logic change → ALL hashes change.
4. **Symbol Graph**: Extracts `Calls`, `Imports`, and `Uses` edges for impact radius analysis.
5. **Limitations**: Breaks on Unicode control chars; no stdin/stdout piping; file-only input.

---

## 5. AGENT PROTOCOL (FOR AI AGENTS)

AI Agents (like Antigravity) MUST adhere to these protocols when interacting with Vantage:

### Boundary Rules:
- **Never modify** files outside the project root.
- **Never edit** dependencies or external toolchains.

### Safety Layer:
1. **Lazy Hydration**: Do NOT read raw file content by default. Call `vantage verify` first.
2. **Structural Guard**: If a file has `@epistemic` markers, treat it as "Locked". Any `normalized_hash` change must be flagged for impact radius review.
3. **Impact Radius**: Use `vantage graph` to identify affected callers BEFORE modifying a function.

### Prohibitions:
- **No server/daemon**: Do not attempt to run Vantage as a persistent service.
- **No inference**: Do not "guess" the meaning of hashes; use the `diff` command for drift reporting.

---

---

## 6. EXPERIMENTAL MODULES & AI TESTING

### Experimental Status (v1.2.4)
- **Verify**: STABLE. Core structural extraction is locked.
- **Graph/Diff/Seal**: EXPERIMENTAL. These modules are open for AI testing in v1.2.4 to gather friction data for the next release.

### AI Reasoning Protocol (v1.2.4-EXPERIMENTAL)
1. **Evidence over Truth**: AI agents must not assume a `vantage` output is the final "Ground Truth". Use it as a structural signal to inform your next research step.
2. **False Positives**: Be prepared for noise in `diff` and `graph` outputs. If a signal contradicts your source code reading, log the friction using `kit learn --kind note --tag decision`.

---

## 7. ROADMAP (v1.2.5)

1. **Deterministic Normalization**: Hardening AST S-expression mapping for a wider range of Rust/Python patterns.
2. **Global Anchor Resolution**: Moving beyond local file analysis to resolve `@epistemic` anchors across the entire workspace.
3. **Forensic Report UI**: Implementing the `vantage report` command for human-readable structural summaries.

---

## 8. TECHNICAL RELEASE SUMMARY (v1.2.4)

### 🧠 Định nghĩa một câu

**Vantage v1.2.4 = Production-ready Memory Integrity Engine (Rust 2024) cho hệ thống Kit.**

Không phải CLI phụ.
Không phải tool debug.

Mà là:

> **Forensic sensor cho bộ nhớ AI.**

---

### 📍 Vai trò của Vantage trong hệ thống

#### Kiến trúc tổng thể hiện tại

```
Agent
   ↓
kit (Python 3.14 runtime)
   ↓ subprocess
kit-vantage (Rust 2024 engine)
   ↓ read-only
.kit/local_brain.db
   ↓
Git
```

Mapping rõ:

| Layer       | Role              |
| ----------- | ----------------- |
| **Kit**     | Memory Brain      |
| **Vantage** | Integrity Sensor  |
| **Git**     | Time Kernel       |
| **Agent**   | Decision Executor |

Đây là **kiến trúc rất sạch**.

---

### 🏗️ Kiến trúc nội bộ Vantage v1.2.4

#### Core Pipeline

```
SQLite (read-only)
        ↓
baked_observations (VIEW)
        ↓
normalize(content)
        ↓
SHA256(structural_hash)
        ↓
verify integrity
```

Không write DB.
Không mutate state.

→ **pure verifier**

---

### 📦 Command Surface

Hiện tại bạn đã có:

```bash
kit-vantage verify-memory
kit-vantage verify-memory -d
kit-vantage verify-memory -j
kit-vantage verify-memory -d -j
kit-vantage benchmark
```

Đây là **production CLI surface**.

---

### 🔍 `verify-memory` — Core Function

#### Basic Mode

```bash
kit-vantage verify-memory
```

Kiểm tra:

| Check           | Mục đích              |
| --------------- | --------------------- |
| Hash integrity  | structural truth      |
| Record validity | corrupted data detect |

Output ví dụ:

```
Records scanned: 119
Valid hashes:   119
Invalid hashes: 0

✅ INTEGRITY OK
```

---

#### `--deep` Mode — Full Forensic Check

```bash
kit-vantage verify-memory -d
```

Kiểm tra:

| Check           | Meaning                  |
| --------------- | ------------------------ |
| Hash integrity  | content hash correctness |
| Orphan nodes    | graph integrity          |
| Index integrity | SQLite index validity    |
| SQLite health   | file integrity           |

Output:

```
Hash integrity:   OK
Orphan nodes:     OK
Index integrity:  OK
SQLite health:    OK

✅ Overall: SAFE
```

Đây là:

> **real integrity verification**
> không phải mock.

---

#### `--json` Output — CI/CD Ready

```bash
kit-vantage verify-memory -d -j
```

Output dạng:

```json
{
  "records": 119,
  "valid_hashes": 119,
  "invalid_hashes": 0,
  "orphan_nodes": 0,
  "index_ok": true,
  "sqlite_ok": true,
  "status": "SAFE"
}
```

Ý nghĩa:

→ dùng trực tiếp trong:

* CI pipeline
* deployment gate
* automated audits

---

### ⚡ `benchmark` — Performance Capability

```bash
kit-vantage benchmark
```

Kết quả hiện tại:

```
Records:      119
Time:         1.82 ms
Speed:        65,467 records/sec
```

Ý nghĩa thật:

> Vantage đủ nhanh cho production-scale memory.

Không còn là bottleneck.

---

### 🔐 Structural Hash System

#### Algorithm

```
normalize(content)
        ↓
SHA256
        ↓
structural_hash
```

Normalization rule:

```
trim()
lowercase()
newline → space
collapse whitespace
```

Quan trọng:

| Feature                 | Status |
| ----------------------- | ------ |
| Python ↔ Rust alignment | ✅      |
| Deterministic hashing   | ✅      |
| Cross-language stable   | ✅      |

Đây là **mốc rất lớn**.

---

### 🧱 SQLite Truth Layer Integration

Vantage đọc từ:

```
.kit/local_brain.db
```

Thông qua:

```
baked_observations (VIEW)
```

Filter:

```
is_active = 1
is_baked = 1
superseded_at IS NULL
```

Ý nghĩa:

> chỉ đọc **truth layer**, không đọc draft.

Rất đúng thiết kế.

---

### 🔄 Migration Capability (v1.2.3 → v1.2.4)

Đã hoàn thành:

```
Old DB → New DB
```

Kết quả thực:

```
Records scanned: 119
Valid hashes:   119
Invalid hashes: 0
```

Ý nghĩa:

> Memory legacy đã được preserve.

Đây là điều cực quan trọng về **data continuity**.

---

### 🧪 Deep Integrity Results (Production Baseline)

Hiện trạng:

```
Total observations: 119
Valid hashes:      119
Orphan nodes:      0
Duplicate hashes:  1 (acceptable)
```

Đánh giá:

```
✅ Graph integrity OK
✅ Hash integrity OK
⚠ Duplicate minor (non-critical)
```

Đây là:

> **production-safe baseline**

---

### 🧠 Triết lý thiết kế Vantage

Một câu:

> **Read-only forensic engine.**

Không:

* mutate
* repair
* write

Chỉ:

```
observe
verify
report
```

Repair thuộc về:

```
kit doctor
```

Không phải Vantage.

Đây là **separation of concerns rất đúng**.

---

### 🧭 Vai trò trong Daily Workflow

Bạn đã định nghĩa:

```bash
kit recall
kit flow run
kit learn
kit-vantage verify-memory
```

Vai trò Vantage:

```
Integrity gate
```

Không phải runtime.
Không phải planner.

→ **verifier**

---

### 📅 Vai trò trong Weekly / Monthly Cycle

#### Weekly

```bash
kit doctor
kit stats
kit-vantage verify-memory -d
```

Mục tiêu:

```
detect drift
```

---

#### Monthly

```bash
kit-vantage benchmark
```

Mục tiêu:

```
performance sanity
```

---

### 📍 Vantage v1.2.4 — Capability Matrix

| Capability                   | Status |
| ---------------------------- | ------ |
| SQLite read-only             | ✅      |
| Hash verification            | ✅      |
| Deep integrity check         | ✅      |
| JSON output                  | ✅      |
| Benchmarking                 | ✅      |
| CI integration ready         | ✅      |
| Cross-language deterministic | ✅      |

Đây là:

> **production-grade tool**

---

### 🔮 Vantage v1.2.5 — Hướng phát triển tự nhiên

Không bắt buộc — nhưng rất hợp lý.

#### Drift Detection

```bash
kit-vantage diff file.rs
```

So:

```
current vs sealed
```

#### Graph Visualization

```bash
kit-vantage graph
```

Output:

```
dependency graph
```

#### Policy Enforcement

```bash
kit-vantage verify --enforce
```

Ví dụ:

```
no orphan nodes allowed
```

---

### 🧭 Tổng kết thật sự

#### Một câu kỹ thuật

> **Vantage v1.2.4 là một Rust-based deterministic memory integrity engine hoạt động ở chế độ read-only trên SQLite truth layer của Kit.**

---

#### Một câu kiến trúc

> **Vantage là cảm biến cấu trúc của bộ não phần mềm.**

---

#### Một câu thực tế

Bạn đã xây xong:

# ✅ **Memory Integrity Engine thật**

Không phải concept.
Không phải prototype.
Mà là:

> **production-grade subsystem.**

---

*v1.2.4 — EXPERIMENTAL MODE ENABLED. Single Source of Truth Under Stress Test.*
