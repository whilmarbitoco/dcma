# Design Consideration: Attention Mechanism in DCMA

**Date:** June 19, 2026
**Status:** Under consideration

---

## The Question

Should DCMA incorporate attention mechanisms and/or backpropagation as seen in modern deep learning systems?

---

## Attention Mechanism

### Verdict: Potentially useful — with constraints

Attention as a *computational pattern* is powerful and well-established. The constraint is that it must remain **deterministic** — no learned attention weights.

### Where it could strengthen DCMA

1. **dcma-retrieval** — Attention-weighted retrieval could improve relevance scoring when pulling from the memory graph, replacing or augmenting pure beam search.
2. **dcma-activation** — Attention-based activation spreading could replace or augment current fixed-point iteration, potentially converging faster.
3. **dcma-consolidation** — Attention over graph neighborhoods could improve motif detection.

### Implementation constraint: deterministic attention only

Attention weights must be **derived analytically**, not learned:

- Graph topology (edge weights, path distance)
- Contradiction pressure scores
- Entropy gradients
- Relation confidence scores

**Not allowed:** learned attention via gradient descent. This would break the formal convergence proofs and determinism guarantees that define DCMA.

---

## Backpropagation

### Verdict: Not recommended

Backpropagation is fundamentally at odds with DCMA's core design philosophy.

### Why it doesn't fit

| DCMA Principle | Backpropagation |
|---|---|
| Deterministic, reproducible cognition | Stochastic gradient descent, sensitive to initialization |
| Formally provable properties | No convergence guarantees for non-convex loss |
| Interpretable memory | Opaque weight matrices |
| Discrete symbolic reasoning | Continuous, sub-symbolic representations |
| Real-time cognitive tick cycle | Requires batch training, offline epochs |

### Specific problems

1. **Breaks determinism** — SGD is inherently stochastic (random initialization, mini-batch ordering, dropout). DCMA's value proposition is deterministic cognition.
2. **Breaks formal proofs** — Convergence proofs in activation dynamics (Banach fixed-point), topology (spectral partitioning), and contradiction (pressure equilibrium) assume deterministic dynamics.
3. **Breaks interpretability** — Post-backprop neural weights are not inspectable the way DCMA's atoms and relations are.
4. **Wrong problem** — Backprop is for learning from raw data. DCMA is for structured reasoning over known knowledge. The ingest pipeline (dcma-ingest) handles NL → structured knowledge. That's the correct learning boundary.

---

## Recommended Alternatives for Learning

If DCMA needs to learn and improve over time, use approaches that **preserve determinism**:

1. **Hebbian-style reinforcement** — Already partially in DCMA. Deterministic, local, biologically plausible. Strengthen edges between co-activated atoms.

2. **Bayesian belief updating** — Update confidence scores on atoms and relations based on evidence. Deterministic given the evidence. Formally grounded.

3. **Structure learning** — Instead of tuning weights, learn the graph structure itself. Add/remove edges based on co-occurrence and contradiction resolution outcomes. Symbolic and interpretable.

4. **Parameter optimization at design time** — Activation thresholds, entropy bounds, consolidation criteria could be tuned via grid search or evolutionary optimization during development. The system remains deterministic at runtime.

---

## Bottom Line

| Approach | Fit | Condition |
|---|---|---|
| Attention mechanism | ✅ Good | Must be deterministic (analytically derived weights) |
| Backpropagation | ❌ Poor | Breaks determinism, proofs, and interpretability |
| Hebbian reinforcement | ✅ Good | Already in DCMA, expand it |
| Bayesian updating | ✅ Good | Formally grounded, deterministic |
| Structure learning | ✅ Good | Symbolic, interpretable |
| Design-time optimization | ✅ Good | Keep runtime deterministic |

**Core principle:** DCMA competes on deterministic, verifiable, structured cognition — not on pattern recognition. That's a genuinely valuable niche. Don't dilute it with mechanisms that undermine its foundations.

---

## Update: The Ingest Layer Argument (June 19, 2026)

### The problem this doesn't solve

The current ingest pipeline (dcma-ingest) is shallow. It extracts entities and simple verb-mediated relations from text. Building deep knowledge still requires manual curation or feeding large volumes of text. DCMA has no passive learning — no autonomous absorption of information from documents, web, or data streams.

The original analysis concluded: "backprop is for learning from raw data, DCMA is for structured reasoning." This is correct for the **core**. But it was incomplete — it didn't consider that the **ingest layer** is exactly where statistical learning belongs.

### The hybrid architecture

```
┌─────────────────────────────────────────────────┐
│                  DCMA Core                       │
│  (deterministic, formal, symbolic reasoning)     │
│                                                  │
│  activation │ retrieval │ contradiction │ entropy│
│  consolidation │ topology │ runtime              │
└──────────────────────┬──────────────────────────┘
                       │ clean atoms/relations
              ┌────────┴────────┐
              │   Ingest Layer   │
              │  (learned,        │
              │   statistical)    │
              │                   │
              │  • entity extract │
              │  • relation learn │
              │  • attention      │
              │  • backprop       │
              └───────────────────┘
```

**Two layers with a clean boundary:**

1. **DCMA Core** — stays deterministic, formal, symbolic. No backprop. All proofs hold. Never sees gradients or weight updates.
2. **Ingest Layer** — learned, statistical, continuous. Uses attention + backprop to get better at extracting structured knowledge from unstructured data.

The ingest layer **feeds** the core. It doesn't **become** the core. The core only receives clean atoms and relations.

### Where backprop belongs: dcma-learn (future crate)

A separate `dcma-learn` crate could implement:

- **Attention** — learned attention weights for retrieval scoring and document comprehension
- **Extraction** — learned entity/relation extraction from text (replacing/augmenting the regex/heuristic approach in dcma-ingest)
- **Feedback loop** — uses core signals as training signal:
  - Did extracted atoms survive contradiction checks?
  - Did retrieval return useful results using these atoms?
  - Did consolidation merge them into stable structures?
- **Adapter** — converts learned output into clean atoms for dcma-core

### What this enables

```bash
# Current (manual):
dcma ingest "Neural Networks are a subset of Machine Learning."

# With learned ingest (autonomous):
dcma learn --from ./textbook.pdf
dcma learn --from https://en.wikipedia.org/wiki/Neural_network
dcma learn --from ./research_papers/
```

The core stays clean and deterministic. The ingest layer gets smarter over time. The boundary is atoms/relations — the core never knows or cares how they were produced.

### Revised verdict

| Layer | Backprop | Attention | Determinism |
|---|---|---|---|
| dcma-core | ❌ Never | ✅ Deterministic only | Preserved |
| dcma-learn | ✅ Yes | ✅ Learned OK | Not required |

The original conclusion — "backprop doesn't belong in DCMA" — was correct for the core. The revision is: **backprop belongs in the ingest layer, as a separate crate with a clean boundary.** This makes DCMA less manual without sacrificing what makes it valuable.
