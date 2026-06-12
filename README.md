# Vantage (v1.2.5)
Reality Operating System. Converts inference into deterministic state.

## Architecture
1. **Engine (`kit-vantage`)**: Parse, graph, and verify codebase invariants.
2. **Runtime (`crates/`)**: 
   - `daemon` & `client`: P2P state routing.
   - `pek` & `trust`: Cryptographic Proof Enforcement.
   - `vfp` & `prn`: Cognitive consensus.
   - `benchmark`: IAR & Reality Yield metrics.

## Docs
- `VANTAGE_CONTRACT.md` (APIs)
- `VANTAGE_SPEC.md` (Math)
- `AGENTS.md` (Agent Loop)
