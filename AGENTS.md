# Vantage Agent Contract (v1.2.3)

## 🛡️ Boundary Rules (Agent Safety Layer)
- **NEVER modify files outside the current repository.**
- **NEVER edit dependencies or external toolchains (e.g., kit core).**
- If an issue is found in external systems, **REPORT it via `kit learn`** instead of fixing it.

## ⚡ Token Efficiency (Token Detox)
- **ALWAYS call `vantage-verify <path>` before modifying any code.**
- **TRUST StructuralSignal over assumptions.** Do not read entire files to guess structure.
- **IF CLI fails, STOP and report.** DO NOT fallback to guessing or raw scanning.

## 🧠 Role
Vantage is a stateless structural sensor. It does not interpret meaning — only structure.
All reasoning and long-term memory must happen in the host (**kit**).
