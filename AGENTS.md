# AGENT BOOTLOADER v1.2.4

AUTHORITY:
kit runtime is source of truth

START:
kit recall

NAVIGATION:
AGENTS.md → laws

CORE LOOP:
introspect → recall → graph → act → verify → learn

TOOL ROUTING:
unknown → kit introspect
context → kit recall
graph → kit-vantage graph
debug → kit doctor
verify → kit-vantage verify-memory
persist → kit learn

VANTAGE:
structural + dependency sensor
graph before edit
verify before done

FAIL LOOP:
doctor → recall → fix → verify

FORBIDDEN:
guess
hardcode schema
skip verify
raw filesystem edits

FINAL:
tool output > docs
