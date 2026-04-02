# .kit Reference Guide (v1.2.3 STABLE)

This guide documents the active CLI and Python API surface for `.kit` v1.2.3.
Use this file when an agent needs exact command syntax after reading [../AGENTS.md](../AGENTS.md), [architecture.md](architecture.md), and [playbook.md](playbook.md).

## Runtime Support

`.kit` v1.2.3 supports Python `3.14.x`.

## Python API

```python
from pathlib import Path
import kit.api as api

api.init_kernel(Path(".kit/brain.db"))
```

### Core Functions

- `init_kernel(db_path: Path | None = None) -> None`
- `learn(uid, content, kind="observation", importance=0.5, metadata=None, layer="episodic", namespace="shared", agent_id=None, supersede_id=None, scope=None, to_global=False, symbol=None, structural_hash=None, skip_render=False) -> int`
- `search(query, limit=15, at=None, agent_id=None, fast=False) -> list[Any]`
- `recall(entities, limit=15, at=None, agent_id=None, here=False, symbol=None, fast=False) -> list[Any]`
- `recall_with_assessment(entities, limit=15, at=None, agent_id=None, here=False, symbol=None, fast=False) -> RankingAssessment`
- `export_prompt(entities, limit=3, budget=200) -> str`
- `reflect(diff_text, scope=None) -> Any`
- `preflight_check(commit_msg, strict=False) -> dict[str, Any]`

### Assessment States

- `HIGH_CONFIDENCE`
- `AMBIGUOUS`
- `WEAK_SIGNAL`
- `EMPTY`

## CLI

Top-level command surface:

```text
kit {init,learn,search,recall,context,reflect,blame,where,link,stats,bump,promote,doctor,render,label,watch,preflight}
kit-agent {run,ask,recall,status,stats,reset-metrics}
```

### Initialization

```bash
kit --version
kit init
kit init --force
kit where
```

Startup guardrail:

- In initialized projects, run `kit recall` exactly as written.
- Do not run `python kit.py recall` unless you are inside the `memory_share_kit` source repo.
- Do not leave an unmatched trailing quote after `kit recall`, or PowerShell may appear to hang while waiting for input.
- `kit --version` prints the active CLI contract version.
- `kit init --force` only resets kit-managed artifacts: `.kit/`, `AGENTS.md`, `docs/reference.md`, and `scripts/kitf.ps1`.

### Learn

Store project or global memory.

```bash
kit learn --uid auth --tag invariant --content "Auth tokens must not be logged"
kit learn --uid cache --tag decision --content "Use SQLite for caching" --symbol cache_layer
kit learn --uid ui --tag note --content "Prefer local file logging for quick diagnostics"
kit learn --uid architecture_v1_2_3 --tag invariant --content "..." --no-render
kit learn --global --tag decision --content "Vantage requires anchors for signals"
kit learn --auto --content "provider discovery falls through sequential TCP checks"
kit learn --file observations.json
```

Supported flags:

- `--uid`: node identity
- `--kind`: observation kind
- `--content`: text to store
- `--importance`: 0.1 to 1.0 weight
- `--supersede`: mark an older observation as superseded
- `--tag`: `invariant`, `decision`, `preference`, `note`, or `legacy`
- `--layer` or `-l`: `working`, `episodic`, `semantic`, or `procedural`
- `--global`: write to the global brain
- `--auto`: let v1.2.3 auto-route local versus global
- `--namespace`: namespace such as `shared` or `agent:name`
- `--scope`: explicit folder scope
- `--agent-id`: attribution
- `--symbol`: anchor to a code symbol
- `--hash`: structural hash for anchored facts
- `--no-render`: skip manifest rendering
- `--file`: load observations from JSON

### Search And Recall

Read memory back out.

```bash
kit recall auth
kit recall cache --here --symbol cache_layer
kit recall architecture --query Vantage --with-global
kit context --limit 5
kit search SQLite --limit 10
```

Supported `recall` and `context` flags:

- `entities`: zero or more entity ids for `recall`
- `--limit`: maximum number of items
- `--at`: temporal snapshot
- `--agent-id`: agent-aware ranking boost
- `--here`: bias to the current directory scope
- `--symbol`: focus recall on one symbol
- `--query`: keyword filter within recalled context
- `--with-global`: include global brain facts
- `--fast`: skip heavier ranking passes

### Governance

```bash
kit reflect --mode advisory --here
kit reflect changed_file.py --json
kit preflight -m "check invariants"
git diff --cached | kit preflight -m "docs: update bootloader"
kit blame validate_token
```

Supported `reflect` flags:

- optional file path
- `--mode`: `strict`, `advisory`, or `silent`
- `--strict`: legacy shortcut for strict mode
- `--json`: emit JSON
- `--scope`: explicit scope
- `--here`: use current directory scope

Supported `preflight` flags:

- `-m` or `--message`: commit message to evaluate
- `--mode`: `strict`, `advisory`, or `silent`
- `--strict`: legacy shortcut for strict mode
- `--json`: emit JSON

### Maintenance

```bash
kit stats
kit bump 42
kit promote --threshold 5
kit link --src auth_layer --dst token_rules --rel DEPENDS_ON --weight 1.0
kit doctor --mode safe
kit doctor --check-agents
kit doctor --reset-cloud
kit label --id 42 --correct GLOBAL
kit render
kit watch --json
kit watch
```

Other command notes:

- `kit stats`: show project and global brain counts
- `kit bump <id>`: reinforce one observation
- `kit promote --threshold N`: move heavily used episodic memories to semantic
- `kit link`: create a semantic edge between nodes
- `kit doctor`: run safe or aggressive hygiene plus optional agent checks
- `kit render`: regenerate `.kit/context` and `AGENTS.md`
- `kit label`: log routing feedback for v1.2.4 training
- `kit watch`: stream cognitive events

## kit-agent

```bash
kit-agent status
kit-agent run "Design the cache layer"
kit-agent run "Implement payment flow" --provider local
kit-agent run "Check governance drift" --type critical --mode advisory
kit-agent ask "Implement a login logger." --json
kit-agent recall auth provider_discovery --limit 5
kit-agent stats
kit-agent reset-metrics
```

Supported `kit-agent run` and `ask` flags:

- `task`: the requested work
- `--type`: `general`, `simple`, `refactor`, or `critical`
- `--mode`: `strict`, `advisory`, or `silent`
- `--provider`: force `local`, `gemini`, `mock`, or another configured provider
- `--json`: emit machine-readable output

Supported `kit-agent recall` flags:

- `entities`: one or more entities or tags
- `--limit`: maximum item count

## Utility Scripts

```bash
python scripts/archive_v123/smoke_test_gemini.py
python scripts/archive_v123/smoke_test_full_local_gemini.py
python scripts/archive_v123/run_stress_test.py
scripts/kitf.ps1
```

## Troubleshooting

### kit-agent Provider Discovery Latency

Older `kit-agent` flows may experience long delays when provider discovery falls through sequential TCP checks. When that happens, use one of these supported workarounds:

#### Force a Healthy Provider

Bypass discovery and target a known working provider directly.

```bash
kit-agent ask "Your task" --provider gemini
```

#### Refuse Local Discovery Explicitly

If you do not run a local Jan or compatible local LLM endpoint, point discovery at a refusal port so the TCP stack fails immediately instead of waiting for a timeout.

```bash
# Unix/macOS/Linux
export JAN_BASE_URL="http://127.0.0.1:1"

# Windows (PowerShell)
$env:JAN_BASE_URL="http://127.0.0.1:1"
```

#### Verify Provider Health

```bash
kit-agent status
```

These workarounds preserve architecture lock while improving response time.

### Agent Runtime Guarantees

- Max repair loop attempts: `3`
- Local fallback is preferred when healthy cloud providers are unavailable
- Capacity failures trigger immediate cooldown-aware fallback
- Prompt injection uses the `.kit` assessment contract instead of raw retrieval alone
- Output contract is JSON with `decision`, `reason`, and `confidence`
- Exit codes are standardized at the `kit-agent` surface: `PASS/WARN=0`, `BLOCK=1`, `ERROR=2`

## Locked Prompt Export Contract

- Maximum Top-K memories: `3`
- Empty export returns an empty string
- Export uses compact first-line rendering
- Prompt budget defaults to approximately `200`

## See Also

- [README.md](../README.md)
- [architecture.md](architecture.md)
- [playbook.md](playbook.md)
- [integrations/vantage.md](integrations/vantage.md)

---

*Last Updated: 2026-03-29 | Version: v1.2.3 STABLE | Status: SEALED*
