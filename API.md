# DCMA HTTP API Reference

Start the server: `dcma serve --port 3000`

All endpoints return JSON. Default port: `3000`.

---

## Health and Status

### GET /health
Returns service status.

Response: `{"status": "ok", "version": "0.1.0"}`

### GET /state
Returns current cognitive state summary.

Response: `{"nodes": 42, "edges": 17, "tick": 5}`

### GET /stats
Returns detailed graph statistics.

```json
{
  "atom_count": 42,
  "relation_count": 17,
  "tick": 5,
  "degree_distribution": {"0": 3, "1": 12, "2": 8},
  "diameter": 4
}
```

---

## Atoms

### GET /atoms
List atoms with optional search filter.

Query params: q (search), limit (default 20), offset (default 0)

Response:
```json
{
  "atoms": [{"id": "...", "atom_type": "Concept", "name": "Machine Learning", ...}],
  "total": 42
}
```

### GET /atoms/:id
Get a single atom. Returns 404 if not found.

### POST /atoms
Create a new atom.

Body: `{"type": "Concept", "name": "Neural Networks", "attributes": {}}`

Response: AtomId string.

### DELETE /atoms/:id
Delete an atom. Returns 404 if not found.

---

## Relations

### POST /relations
Create a relation.

Body: `{"src": "<UUID>", "dst": "<UUID>", "type": "InstanceOf"}`

Response: `{"status": "created"}`

---

## Cognitive Operations

### POST /tick
Execute one cognitive tick.

Body: `{"sources": ["<UUID>"], "query": ["<UUID>"]}`

Response: TickResult with convergence info.

### POST /query
Run retrieval query.

Body: `{"nodes": ["<UUID>"]}`

Response: CognitiveField with relevance scores.

---

## Natural Language Ingest

### POST /ingest
Extract entities and relations from text.

Body: `{"text": "Neural Networks are a subset of Machine Learning."}`

Response: `{"atoms_created": 3, "atom_ids": [...], "relations_found": 2}`

---

## Export

### GET /export
Export full cognitive state as JSON.
