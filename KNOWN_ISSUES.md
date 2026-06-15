# DCMA — Known Issues & Usability Gaps

> Auto-generated assessment of current blockers preventing end-to-end usability.
> Last updated: 2026-06-15

## Summary

All 185 tests pass across 12 crates. The CLI, API, and PostgreSQL persistence all build and run. However, the system is a well-architected **framework** but not yet a usable **application**. Below are the specific gaps.

---

## 🔴 Critical (No real usage possible without these)

### 1. No Data Ingestion Pipeline
- Atoms must be manually created via CLI (`dcma insert --type concept --name "foo"`) or API POST
- No way to feed text, documents, or structured data into the engine
- No NLP pipeline to extract entities, concepts, or relations from raw text
- **Impact:** The engine has no way to learn or build knowledge automatically

### 2. No Embedding / Semantic Layer
- Retrieval engine does graph traversal only — no vector embeddings
- No semantic similarity search (can't find "similar concepts")
- No way to map natural language queries to existing atoms
- **Impact:** The `query` endpoint only works with known AtomIds, not natural language

### 3. No Contradiction Detection on Real Data
- Contradiction engine works mathematically (pressure dynamics, equilibrium)
- No NLP pipeline to detect contradictions in actual content
- **Impact:** Contradictions must be manually specified; the engine can't discover them

---

## 🟡 High Priority (Degraded experience)

### 4. No Adaptive Tick Scheduling
- Cognitive tick runs all subsystems sequentially every time
- No priority queue, no adaptive scheduling based on graph size or change rate
- No way to handle large graphs efficiently
- **Impact:** Performance degrades linearly with graph size; no optimization for sparse updates

### 5. No Visualization or Inspection Tool
- State can be exported as JSON but there's no way to visualize:
  - Activation patterns across the graph
  - Topology partitions / community structure
  - Weight distributions over time
  - Contradiction pressure hotspots
- **Impact:** Debugging and understanding cognitive state requires reading raw JSON

### 6. No Configuration System
- All engine parameters are hardcoded defaults in `EngineConfig::default()`
- No config file support (YAML/TOML), no environment variable overrides
- No way to tune the engine for different use cases without recompiling
- **Impact:** Can't adapt the engine behavior to different domains or workloads

---

## 🟢 Polish (Nice to have)

### 7. No Documentation or Examples
- No README in the repo root
- No usage guide or example workflows
- No API documentation beyond code comments
- Phase docs exist (`docs/PHASE_*.md`) but are implementation notes, not user guides

### 8. Binary Name Mismatch
- CLI binary is `dcma-cli` instead of `dcma`
- Cargo.toml `name = "dcma-cli"` should be `name = "dcma"` for the binary to match user expectations

### 9. Dead Code Warnings in `dcma-pg`
- 6 methods in `PgStorage` are never used: `save_tick`, `load_ticks`, `save_atom`, `delete_atom`, `save_relation`, `delete_relation`
- These are the granular per-entity operations — the current save/load uses bulk `save_state`/`load_state` instead
- Not a bug, but indicates incomplete API surface usage

### 10. `PgRuntime` Not Wired Into CLI or API
- The `PgRuntime` wrapper (combines `Runtime` + `PgStorage` with auto-persist) exists but is only used in integration tests
- The CLI uses file-based `Storage` (JSON), not PostgreSQL
- The API uses in-memory `Runtime` with no persistence
- **Impact:** The PostgreSQL backend works but isn't accessible through any user-facing interface

---

## Architecture Notes

### What Works
| Component | Status | Notes |
|---|---|---|
| dcma-core | ✅ | CognitiveState, atoms, relations, contradictions |
| dcma-activation | ✅ | Signal propagation with convergence proof |
| dcma-retrieval | ✅ | Beam search, relevance scoring |
| dcma-contradiction | ✅ | Pressure dynamics, equilibrium model |
| dcma-entropy | ✅ | Shannon entropy, regulation triggers |
| dcma-consolidation | ✅ | VF2 motif detection, crystallization |
| dcma-topology | ✅ | Spectral partitioning, hub splitting |
| dcma-runtime | ✅ | Phi cognitive evolution, tick orchestration |
| dcma-storage | ✅ | JSON file persistence |
| dcma-pg | ✅ | PostgreSQL backend, 8 tables, trigram FTS |
| dcma-distributed | ✅ | HLC clocks, CRDT merge protocol |
| dcma-api | ✅ | Axum HTTP API, 8 endpoints |
| dcma-cli | ✅ | Clap CLI, 9 commands |

### Test Coverage
- **185 tests passing** across all 12 crates
- Zero failures, zero flaky tests (after `#[serial]` fix)
- Integration tests cover `PgStorage` save/load cycle
- API tests cover all 8 endpoints

### Infrastructure
- PostgreSQL: dedicated `dcma` database on `llm-ciel-memory-qk6cz9:5432`
- Repo: `github.com/whilmarbitoco/dcma` (public)
- Branch: `main`, up to date with remote
