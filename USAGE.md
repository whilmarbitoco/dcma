# DCMA — Usage Guide

**Deterministic Cognitive Memory Architecture**

A working reference implementation of a deterministic cognitive architecture where memory and reasoning are first-class, formally specified, and fully observable.

## Installation

```bash
git clone https://github.com/whilmarbitoco/dcma.git
cd dcma
cargo build --release
cargo test --workspace
```

## Quick Start

### 1. Initialize

```bash
dcma init
```

### 2. Add atoms

```bash
dcma add --type Concept --name "Machine Learning"
dcma add --type Concept --name "Neural Networks"
dcma add --type Entity --name "Backpropagation"
```

### 3. Connect with relations

```bash
dcma connect --src <UUID> --dst <UUID> --type InstanceOf
dcma connect --src <UUID> --dst <UUID> --type Causes
```

### 4. Ingest natural language

```bash
dcma ingest "Neural Networks are a subset of Machine Learning. Backpropagation causes weight updates."
```

### 5. Run a cognitive tick

```bash
dcma tick
```

### 6. Search and inspect

```bash
dcma search "neural"
dcma list --limit 20
dcma get <UUID>
dcma stats
```

### 7. Start HTTP API

```bash
dcma serve --port 3000
```

## CLI Reference

```
dcma <COMMAND> [OPTIONS]

Commands:
  init                    Initialize a new cognitive state
  add                     Insert a new atom (--type, --name)
  search <QUERY>          Search atoms by name (substring)
  list                    List all atoms (--limit, --offset)
  get <ID>                Show details of a specific atom
  remove <ID>             Delete an atom
  connect                 Create a relation (--src, --dst, --type)
  ingest <TEXT>           Ingest natural language text
  query                   Run retrieval query (--nodes UUID,UUID,...)
  tick                    Execute cognitive tick (--sources UUID,UUID,...)
  stats                   Show graph statistics
  export                  Export state to JSON (--file path)
  import <FILE>           Import state from JSON
  serve                   Start HTTP API server (--port 3000)

Global Options:
  --path <DIR>            State directory (default: ".")
  --json                  Output as JSON

Atom Types:    Concept, Event, Entity, Procedure, Abstraction
Relation Types: Causes, Prevents, Enables, Triggers, DerivesFrom,
                 PartOf, InstanceOf, Specializes, Generalizes,
                 Contains, AbstractionOf, Implies, Negates,
                 Supports, Contradicts, Precedes, Follows,
                 CoOccursWith, PersistsWith, Reinforces,
                 Inhibits, Activates, Suppresses, Focuses
```

## State Persistence

State stored as JSON in `<path>/dcma_state.json`. Every mutation auto-saves.

```bash
dcma export --file backup.json
dcma import backup.json
```

## Architecture

13 crates, 206 tests. See repository README for full details.
