# Deterministic Cognitive Memory Architecture (DCMA)

## Formal Specification v1.0

**Author:** Whilmar M. Bitoco
**Affiliation:** IICT, Aces Tagum College Inc.
**Date:** June 2026

---

## Abstract

This document provides the complete mathematical formalization of DCMA — a deterministic topology-driven cognitive runtime for persistent autonomous intelligence. DCMA replaces probabilistic semantic compression (embeddings, RAG, vector search) with dynamic symbolic topology: meaning emerges from graph geometry, memory is continuously evolving state, retrieval is state reconstruction, contradiction is native structural tension, abstraction is continuous compression, and attention is energy propagation.

Every subsystem is defined with exact equations, algorithmic specifications, computational complexity bounds, and determinism proofs.

**Keywords:** cognitive architecture, dynamic graph systems, deterministic memory, symbolic reasoning, topology-driven cognition

---

## Table of Contents

1. [Foundational Axioms (The Six Laws)](#1-foundational-axioms)
2. [Formal System Definition](#2-formal-system-definition)
3. [Activation Dynamics — Full Specification](#3-activation-dynamics)
4. [Retrieval Engine — Algorithm & Complexity](#4-retrieval-engine)
5. [Contradiction Engine — Equilibrium Model](#5-contradiction-engine)
6. [Entropy Regulation Engine](#6-entropy-regulation-engine)
7. [Consolidation Engine](#7-consolidation-engine)
8. [Symbol Engine — Canonicalization](#8-symbol-engine)
9. [Topology Engine](#9-topology-engine)
10. [Determinism Proof](#10-determinism-proof)
11. [Distributed Cognition Protocol](#11-distributed-cognition)
12. [Complexity Analysis](#12-complexity-analysis)

---

## 1. Foundational Axioms

### Law 1 — Cognition Is Topology

**Axiom.** Meaning is a function of graph geometry, not independent content.

Let $G(t) = (V(t), E(t), W(t))$ be the cognitive graph at time $t$.
For any node $v \in V$, its semantic content is:

$$M(v, t) = f\left(\mathcal{N}_k(v), \rho(v), \tau(v), \phi(v, t), \sigma(v, t)\right)$$

where:
- $\mathcal{N}_k(v)$ = the $k$-hop neighborhood of $v$
- $\rho(v) = \frac{|\{e \in E : v \in e\}|}{|V|}$ = local relation density
- $\tau(v)$ = set of causal path types rooted at $v$
- $\phi(v, t)$ = temporal reinforcement integral (Section 3)
- $\sigma(v, t)$ = activation state (Section 3)

**Corollary.** A graph with isolated nodes has zero semantics. Semantics require topology.

---

### Law 2 — Memory Is Dynamic State

**Axiom.** Memory is not storage. Memory is a trajectory in state space.

The cognitive state at time $t$ is:

$$S(t) = \langle V(t), E(t), W(t), A(t), \mathcal{C}(t), \mathcal{H}(t) \rangle$$

where:
- $V(t)$ = node set
- $E(t)$ = edge set
- $W(t) : E \to \mathbb{Q}^+$ = edge weight function
- $A(t) : V \to \mathbb{Q}^{\geq 0}$ = activation vector
- $\mathcal{C}(t)$ = contradiction structure
- $\mathcal{H}(t)$ = history of state transitions

Each interaction applies a state transition function:

$$S(t+1) = \Phi(S(t), I(t))$$

where $I(t)$ is the input stimulus at time $t$, and $\Phi$ is the cognitive evolution function composed of all seven engine operations.

**Property.** $\Phi$ is deterministic: identical $S(t)$ and $I(t)$ always produce identical $S(t+1)$. (Proof: Section 10.)

---

### Law 3 — Retrieval Is State Reconstruction

**Axiom.** Retrieval does not search. Retrieval reconstructs an optimal cognitive field.

Given current state $S(t)$ and query context $q$, retrieval solves:

$$G^*(t) = \arg\max_{G_q \subseteq G(t)} R(G_q, S(t), q)$$

where $R$ is the relevance functional defined in Section 4.

**Property.** The result $G^*(t)$ is a subgraph — an activated cognitive field. Not a document list. Not a ranked set of text chunks.

---

### Law 4 — Contradiction Is Native

**Axiom.** Contradictions are structural tension, not corruption.

For any pair of cognitive atoms $a, b \in V$ where $\text{REL}(a, b) = \text{CONTRADICTS}$, both nodes persist. The contradiction structure $\mathcal{C}(t)$ tracks:

$$p_c(t) = \frac{| \text{supp}(a,t) | - | \text{supp}(b,t) |}{| \text{supp}(a,t) | + | \text{supp}(b,t) |}$$

where $\text{supp}(x,t)$ is the support topology for node $x$ at time $t$.

**Definition (Equilibrium).** A contradiction is in equilibrium iff:

$$\left| \frac{dp_c}{dt} \right| < \theta_{\text{eq}} \quad \text{for} \quad \Delta t \geq T_{\text{eq}}$$

where $\theta_{\text{eq}}$ is the equilibrium threshold and $T_{\text{eq}}$ is the stabilization window.

**Property.** Truth is the equilibrium state of competing reinforcement structures. (Full model: Section 5.)

---

### Law 5 — Abstraction Is Compression

**Axiom.** Intelligence accumulation is the crystallization of recurring graph motifs into abstraction nodes.

Let $\mathcal{M}(t)$ be the set of recurring subgraph motifs in $G(t)$. For each motif $m \in \mathcal{M}$ with recurrence count $r(m,t)$:

$$\text{crystallize}(m) \iff r(m,t) \geq \theta_{\text{cryst}} \quad \land \quad \text{stability}(m,t) \geq \theta_{\text{stab}}$$

When crystallization occurs, a new abstraction node $v_{\text{abs}}$ is created with edges to all participating nodes in $m$.

**Property.** Without abstraction, graph entropy increases unbounded. Abstraction is cognitive consolidation. (Full model: Section 7.)

---

### Law 6 — Attention Is Energy Distribution

**Axiom.** Attention is activation energy allocation over topology.

Each node $v \in V$ at time $t$ possesses:

| Property | Symbol | Type | Description |
|----------|--------|------|-------------|
| Activation energy | $A(v,t)$ | $\mathbb{Q}^{\geq 0}$ | Current cognitive activation |
| Reinforcement momentum | $R(v,t)$ | $\mathbb{Q}^{\geq 0}$ | Accumulated reinforcement |
| Salience gravity | $\Gamma(v,t)$ | $\mathbb{Q}^{\geq 0}$ | Attraction strength |
| Contradiction pressure | $C(v,t)$ | $\mathbb{Q}^{\geq 0}$ | Suppression from contradictions |
| Contextual charge | $Q(v,t)$ | $\mathbb{Q}$ | Alignment with current context |

Cognition emerges from energy flow, not explicit query execution.

---

## 2. Formal System Definition

### 2.1 Cognitive Atom

The fundamental unit of cognition is the **Cognitive Atom**:

$$\text{Atom} = \langle \text{id}, \text{type}, \text{canonical}, \text{state}, \text{attrs}, \text{temporal}, \Gamma, \text{conf}, \text{origin}, \mathcal{R}, \mathcal{A}, \mathcal{C} \rangle$$

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique immutable identifier |
| `type` | Enum | CONCEPT, EVENT, ENTITY, PROCEDURE, ABSTRACTION |
| `canonical` | String | Canonicalized symbolic form |
| `state` | Enum | ACTIVE, DORMANT, FROZEN, CONSOLIDATED |
| `attrs` | Map | Key-value attribute store |
| `temporal` | Range | $[t_{\text{created}}, t_{\text{last\_accessed}}]$ |
| $\Gamma$ | $\mathbb{Q}^{\geq 0}$ | Salience gravity |
| `conf` | $[0, 1] \cap \mathbb{Q}$ | Confidence score |
| `origin` | Provenance | Source lineage |
| $\mathcal{R}$ | List | Reinforcement history |
| $\mathcal{A}$ | List | Activation history |
| $\mathcal{C}$ | Set | Contradiction indices |

### 2.2 Relation

Edges are typed, weighted, directed semantic operators:

$$\text{Relation} = \langle \text{id}, \text{src}, \text{dst}, \text{type}, w, \text{stability}, \text{conf}, \text{temporal} \rangle$$

**Relation type taxonomy:**

| Category | Operators |
|----------|-----------|
| Causal | CAUSES, PREVENTS, ENABLES, TRIGGERS, DERIVES_FROM |
| Structural | PART_OF, INSTANCE_OF, SPECIALIZES, GENERALIZES, CONTAINS |
| Logical | IMPLIES, NEGATES, SUPPORTS, CONTRADICTS |
| Temporal | PRECEDES, FOLLOWS, CO_OCCURS_WITH, PERSISTS_WITH |
| Cognitive | REINFORCES, INHIBITS, ACTIVATES, SUPPRESSES, FOCUSES |

### 2.3 Edge Weight Function

$$w : E \to \mathbb{Q}^+ \cap [0, 1]$$

Edge weights evolve via:

$$w(e, t+1) = \text{clamp}_{[0,1]}\left(w(e,t) + \Delta w_{\text{reinforce}} - \Delta w_{\text{decay}} + \Delta w_{\text{contradiction}}\right)$$

where:
- $\Delta w_{\text{reinforce}} = \eta_r \cdot r(e,t)$ (reinforcement increment)
- $\Delta w_{\text{decay}} = \eta_d \cdot w(e,t)$ (proportional decay)
- $\Delta w_{\text{contradiction}} = -\eta_c \cdot c(e,t)$ (contradiction suppression)
- $\eta_r, \eta_d, \eta_c \in \mathbb{Q}^+$ are fixed learning rates
- $\text{clamp}_{[0,1]}(x) = \max(0, \min(1, x))$

---

## 3. Activation Dynamics — Full Specification

### 3.1 State Evolution Equation

The activation vector $A(t)$ evolves as:

$$A_i(t+1) = \sigma\left(\sum_{j \in \mathcal{N}(i)} w_{ji} \cdot A_j(t) + R_i(t) - D_i(t) - C_i(t) + Q_i(t)\right)$$

**Parameter definitions:**

| Symbol | Definition | Computation |
|--------|------------|-------------|
| $A_i(t)$ | Activation of node $i$ at time $t$ | State variable, $A_i(t) \in [0, 1] \cap \mathbb{Q}$ |
| $w_{ji}$ | Propagation weight from $j$ to $i$ | Normalized edge weight: $w_{ji} = \frac{w(e_{ji})}{\sum_{k \in \mathcal{N}(i)} w(e_{ki})}$ |
| $R_i(t)$ | Reinforcement energy | $R_i(t) = \eta_r \cdot \sum_{h=0}^{H-1} \lambda^h \cdot r_i(t-h)$ where $r_i(t)$ is reinforcement events in window $H$, $\lambda \in (0,1)$ is decay factor |
| $D_i(t)$ | Decay sink | $D_i(t) = \eta_d \cdot A_i(t)$ where $\eta_d \in (0,1)$ is decay rate |
| $C_i(t)$ | Contradiction suppression | $C_i(t) = \eta_c \cdot \sum_{j \in \text{contradicts}(i)} A_j(t)$ where $\eta_c \in (0,1)$ |
| $Q_i(t)$ | Contextual charge | $Q_i(t) = \eta_q \cdot \text{sim}_{\text{struct}}(i, q(t))$ where $q(t)$ is current query context, $\eta_q \in \mathbb{Q}^+$ |
| $\sigma$ | Normalization function | $\sigma(x) = \frac{1}{1 + e^{-x}}$ (sigmoid, ensuring $A_i \in [0,1]$) |
| $\mathcal{N}(i)$ | Inbound neighbors | $\{j \in V : (j, i) \in E\}$ |

### 3.2 Convergence Theorem

**Theorem.** The activation dynamics converge to a fixed point for any initial state $A(0)$.

**Proof sketch.** The update function is a contraction mapping on the complete metric space $([0,1]^n, d_\infty)$ because:
1. The weight matrix $\mathbf{W} = [w_{ji}]$ is row-stochastic (rows sum to ≤ 1 after normalization)
2. The decay term $D_i$ provides a contraction factor $(1 - \eta_d) < 1$
3. The sigmoid $\sigma$ is Lipschitz continuous with constant 1
4. By Banach fixed-point theorem, repeated application converges to unique $A^*$

**Convergence rate.** After $T$ iterations:

$$\|A(T) - A^*\|_\infty \leq \frac{\alpha^T}{1 - \alpha} \cdot \|A(1) - A(0)\|_\infty$$

where $\alpha = \max_i \sum_j |w_{ji}| \cdot (1 - \eta_d) < 1$.

**Practical bound.** For typical parameters ($\eta_d = 0.1$, max degree $d_{\max} = 50$), convergence to $\epsilon = 10^{-6}$ requires $T \leq \frac{\ln(\epsilon)}{\ln(\alpha)} \approx 80$ iterations.

### 3.3 Propagation Algorithm

```
Algorithm: ACTIVATE(G, source_nodes, query_context, max_iter, ε)

Input:
  G         = cognitive graph
  source    = set of initially activated nodes
  q         = current query context (subgraph)
  T_max     = maximum iterations (default: 100)
  ε         = convergence threshold (default: 1e-6)

Output:
  A_final   = converged activation vector

1.  Initialize: A_i(0) = 1.0 for i ∈ source, 0.0 otherwise
2.  For t = 0, 1, 2, ..., T_max:
3.    For each node i ∈ V (parallelizable):
4.      inbound ← Σ_{j∈N(i)} w_ji · A_j(t)
5.      R_i    ← η_r · Σ_{h=0}^{H-1} λ^h · r_i(t-h)
6.      D_i    ← η_d · A_i(t)
7.      C_i    ← η_c · Σ_{j∈contradicts(i)} A_j(t)
8.      Q_i    ← η_q · structural_similarity(i, q)
9.      A_i(t+1) ← σ(inbound + R_i - D_i - C_i + Q_i)
10.   If max_i |A_i(t+1) - A_i(t)| < ε: break
11. Return A(t+1)

Complexity: O(T_max · (|V| + |E|))
```

### 3.4 Spontaneous Activation

Without external input, the system exhibits spontaneous resurfacing:

$$A_{\text{spont}}(t) = \sum_{v \in V} \Gamma(v,t) \cdot \xi(v,t)$$

where $\xi(v,t) \sim \text{Bernoulli}(p_{\text{spont}})$ with $p_{\text{spont}} = 10^{-4}$ per tick per node. This models the cognitive phenomenon of spontaneous memory resurfacing and is the mechanism behind "intuition" — activation arising from salience gravity without explicit query.

---

## 4. Retrieval Engine — Algorithm & Complexity

### 4.1 Relevance Functional

Given query context $q$ and candidate subgraph $G_q \subseteq G$:

$$R(G_q, S, q) = \alpha \cdot C(G_q, q) + \beta \cdot \Sigma(G_q) + \gamma \cdot \mathcal{A}(G_q) + \delta \cdot T(G_q) - \lambda \cdot D(G_q)$$

**Term definitions:**

| Term | Symbol | Definition | Range |
|------|--------|------------|-------|
| Contextual coherence | $C(G_q, q)$ | $\frac{1}{|V_q|} \sum_{v \in V_q} \text{sim}_{\text{struct}}(v, q)$ | $[0, 1]$ |
| Salience | $\Sigma(G_q)$ | $\frac{1}{|V_q|} \sum_{v \in V_q} \Gamma(v, t)$ | $[0, 1]$ |
| Activation resonance | $\mathcal{A}(G_q)$ | $\frac{1}{|V_q|} \sum_{v \in V_q} A(v, t)$ | $[0, 1]$ |
| Temporal relevance | $T(G_q)$ | $\frac{1}{|V_q|} \sum_{v \in V_q} e^{-\mu \cdot (t - t_{\text{last}}(v))}$ | $[0, 1]$ |
| Traversal cost | $D(G_q)$ | $\frac{\text{diameter}(G_q)}{\text{diameter}(G)}$ | $[0, 1]$ |

**Coefficient constraints:** $\alpha + \beta + \gamma + \delta + \lambda = 1$, all $\in \mathbb{Q}^{\geq 0}$.

Default values: $\alpha = 0.35$, $\beta = 0.20$, $\gamma = 0.20$, $\delta = 0.15$, $\lambda = 0.10$.

### 4.2 Structural Similarity

$$\text{sim}_{\text{struct}}(v, q) = \frac{|\mathcal{N}_k(v) \cap \mathcal{N}_k(q)|}{|\mathcal{N}_k(v) \cup \mathcal{N}_k(q)|} \cdot \frac{w(\text{path}(v, q))}{\text{diameter}(G)}$$

This is a Jaccard-like measure over $k$-hop neighborhoods, weighted by path strength.

### 4.3 Search Space

The search space is bounded:

$$\mathcal{G}_q = \{G' \subseteq G : \text{dist}(G', q) \leq d_{\max} \land |V'| \leq n_{\max}\}$$

where:
- $d_{\max}$ = maximum traversal depth from query context (default: 4 hops)
- $n_{\max}$ = maximum subgraph size (default: 500 nodes)

### 4.4 Optimization Algorithm

Exact optimization over $\mathcal{G}_q$ is NP-hard (it generalizes the maximum weight connected subgraph problem). We use **bounded beam search**:

```
Algorithm: RETRIEVE(G, q, d_max, n_max, beam_width)

Input:
  G           = cognitive graph
  q           = query context (set of nodes)
  d_max       = max traversal depth
  n_max       = max subgraph nodes
  beam_width  = beam width (default: 20)

Output:
  G*          = optimal activated subgraph

1.  frontier ← { {v} : v ∈ q }          // Initialize with query nodes
2.  best ← ∅, best_score ← -∞
3.  For depth = 1, 2, ..., d_max:
4.    candidates ← ∅
5.    For each partial subgraph g in frontier:
6.      neighbors ← {v ∉ g : ∃u ∈ g, (u,v) ∈ E}
7.      For each n ∈ neighbors where |V_g ∪ {n}| ≤ n_max:
8.        g_new ← g ∪ {n} ∪ edges(g, n)
9.        score ← R(g_new, S, q)
10.       candidates ← candidates ∪ {(g_new, score)}
11.    frontier ← top-k(candidates, beam_width) by score
12.    If max score in frontier > best_score:
13.      best ← argmax(frontier), best_score ← max(frontier)
14.  Activate: propagate activation from best for T_act iterations
15.  Return best

Complexity: O(d_max · beam_width · d_avg · n_max)
  where d_avg = average node degree
```

### 4.5 Retrieval Output

The output is an **activated cognitive field** — a subgraph with per-node activation values:

$$\text{output} = \langle G^*, A_{G^*}, \text{metadata} \rangle$$

where metadata includes: relevance score, source provenance, contradiction flags, and temporal range.

---

## 5. Contradiction Engine — Equilibrium Model

### 5.1 Contradiction Structure

A contradiction is a typed relation $\text{CONTRADICTS} \subseteq V \times V \times \mathbb{Q}$:

$$\mathcal{C}(t) = \{(a_i, b_i, p_i(t)) : (a_i, b_i) \in E \land \text{type}(a_i, b_i) = \text{CONTRADICTS}\}$$

where $p_i(t)$ is the contradiction pressure at time $t$.

### 5.2 Contradiction Pressure Dynamics

For contradiction $c = (a, b, p)$:

$$p(t+1) = p(t) + \eta_p \cdot \left(\frac{A(a,t) - A(b,t)}{A(a,t) + A(b,t) + \epsilon}\right) - \eta_d \cdot p(t)$$

where:
- $\eta_p \in \mathbb{Q}^+$ = pressure accumulation rate
- $\eta_d \in (0,1)$ = pressure decay rate
- $\epsilon = 10^{-8}$ = numerical stability constant

**Interpretation.** Pressure increases when one belief is significantly more activated than its contradictor. Pressure decays over time if both beliefs have equal support.

### 5.3 Equilibrium Definition

**Definition.** A contradiction $(a, b, p)$ is in **equilibrium** at time $t$ iff:

$$\left| p(t) \right| \leq \theta_{\text{eq}} \quad \text{AND} \quad \left| \frac{dp}{dt} \right| < \theta_{\text{rate}} \quad \forall \tau \in [t - T_{\text{eq}}, t]$$

**Default parameters:** $\theta_{\text{eq}} = 0.05$, $\theta_{\text{rate}} = 0.01$, $T_{\text{eq}} = 100$ ticks.

### 5.4 Equilibrium Classification

| State | Condition | Interpretation |
|-------|-----------|----------------|
| Stable equilibrium | $|p| \leq \theta_{\text{eq}}$, low rate | Both beliefs coexist peacefully |
| Dominant equilibrium | $|p| > \theta_{\text{eq}}$, sign stable | One belief is winning |
| Oscillating | $|p| > \theta_{\text{eq}}$, high rate | Active competition, unresolved |
| Resolved | $p > \theta_{\text{resolve}}$ | One belief dominates, other suppressed |

**Resolution threshold:** $\theta_{\text{resolve}} = 0.95$. When exceeded, the suppressed node is marked DORMANT (not deleted — Law 4 requires preservation).

### 5.5 Contradiction Influence on Retrieval

Contradiction pressure modifies the retrieval relevance functional:

$$R_{\text{contrad}}(G_q) = R(G_q) \cdot \left(1 - \frac{1}{|V_q|} \sum_{v \in V_q} C(v,t)\right)$$

Contradicted nodes are deprioritized but never eliminated from results.

---

## 6. Entropy Regulation Engine

### 6.1 Graph Entropy Measure

We define cognitive graph entropy using Shannon entropy over the degree distribution:

$$H(G) = -\sum_{d=0}^{d_{\max}} P(d) \log_2 P(d)$$

where $P(d) = \frac{|\{v \in V : \deg(v) = d\}|}{|V|}$.

**Interpretation:**
- $H \approx 0$: All nodes have similar degree (uniform, low information)
- $H$ high: Diverse degree distribution (rich structure)
- $H$ too high: Hub-dominated, fragile topology

### 6.2 Entropy Bounds

The engine maintains entropy within operational bounds:

$$H_{\min} \leq H(G(t)) \leq H_{\max}$$

**Default bounds:** $H_{\min} = 1.5$, $H_{\max} = 5.0$ (for typical cognitive graphs).

### 6.3 Regulation Operations

| Condition | Operation | Effect |
|-----------|-----------|--------|
| $H > H_{\max}$ | Abstraction promotion | Create abstraction nodes for dense clusters |
| $H > H_{\max}$ | Redundancy collapse | Merge duplicate edges, fuse identical motifs |
| $H < H_{\min}$ | Topology diversification | Create bridging edges between clusters |
| $H < H_{\min}$ | Episodic expansion | Re-activate dormant episodic nodes |
| Any node with $\deg(v) > 5 \cdot \bar{d}$ | Hub splitting | Partition hub into specialized sub-nodes |
| Any node with $A(v,t) < \theta_{\text{dead}}$ for $T_{\text{dead}}$ ticks | Dead-path elimination | Remove node and edges |

**Default thresholds:** $\theta_{\text{dead}} = 10^{-6}$, $T_{\text{dead}} = 10000$ ticks.

### 6.4 Compression Operations

**Episodic Compression.** Collapse sequences of repetitive events:

$$\text{compress}(\{e_1, e_2, \ldots, e_n\}) = e_{\text{rep}} \quad \text{where} \quad \text{sim}(e_i, e_j) > \theta_{\text{compress}}$$

The representative event $e_{\text{rep}}$ stores a count attribute $n$.

**Motif Fusion.** Merge recurring subgraphs:

$$\text{fuse}(\{m_1, m_2, \ldots, m_k\}) = m_{\text{abs}} \quad \text{where} \quad \text{isomorphic}(m_i, m_j) > \theta_{\text{fuse}}$$

The fused abstraction node $m_{\text{abs}}$ preserves aggregate reinforcement and temporal range.

---

## 7. Consolidation Engine

### 7.1 Motif Detection

A **motif** is a connected subgraph pattern occurring more than once. Detection uses a bounded VF2-like subgraph isomorphism:

```
Algorithm: DETECT_MOTIFS(G, min_size, max_size, min_recurrence)

1.  motifs ← ∅
2.  For size = min_size to max_size:
3.    patterns ← enumerate_connected_subgraphs(G, size)
4.    For each unique pattern p (up to isomorphism):
5.      count ← count_occurrences(G, p)
6.      If count ≥ min_recurrence:
7.        motifs ← motifs ∪ {(p, count)}
8.  Return motifs

Complexity: O(|V|^max_size) — bounded by small max_size (default: 5)
```

### 7.2 Crystallization Criteria

A motif $m$ with recurrence $r(m,t)$ crystallizes into an abstraction node when:

$$r(m,t) \geq \theta_{\text{cryst}} \quad \land \quad \text{stability}(m,t) \geq \theta_{\text{stab}}$$

where:

$$\text{stability}(m,t) = 1 - \frac{\text{var}(\{w(e) : e \in m\})}{\text{mean}(\{w(e) : e \in m\}) + \epsilon}$$

**Default thresholds:** $\theta_{\text{cryst}} = 5$, $\theta_{\text{stab}} = 0.7$.

### 7.3 Crystallization Process

```
Algorithm: CRYSTALLIZE(motif m, recurrence r)

1.  v_abs ← CREATE_NODE(type=ABSTRACTION, canonical=canonicalize(m))
2.  For each node v ∈ m:
3.    CREATE_EDGE(v_abs, v, type=ABSTRACTION_OF, w=1.0)
4.  For each edge e ∈ m:
5.    CREATE_EDGE(v_abs, e.dst, type=GENERALIZES, w=mean_weight(m))
6.  v_abs.Γ ← mean({v.Γ : v ∈ m}) · log(r)   // Salience scales with recurrence
7.  v_abs.conf ← stability(m)
8.  v_abs.temporal ← [min(t_created), max(t_accessed)]
9.  Mark m nodes as CONSOLIDATED (not deleted)
```

### 7.4 Hierarchical Abstraction

Abstraction nodes can themselves participate in motifs, enabling multi-level hierarchies:

$$\text{Level 0: } \{postgres, indexing, query\_planning\}$$
$$\text{Level 1: } DATABASE\_ENGINEERING$$
$$\text{Level 2: } SOFTWARE\_INFRASTRUCTURE$$

Maximum abstraction depth: configurable, default 5 levels.

---

## 8. Symbol Engine — Canonicalization

### 8.1 Canonicalization Pipeline

```
Input: raw symbol s
Output: canonical form c*

1.  s₁ ← LOWERCASE(s)
2.  s₂ ← NORMALIZE_UNICODE(s₁)
3.  s₃ ← STEM(s₂)                    // Porter-style stemming
4.  s₄ ← RESOLVE_ALIASES(s₃)        // Lookup alias table
5.  s₅ ← ONTOLOGY_NORMALIZE(s₄)     // Map to ontology standard
6.  c* ← s₅
7.  If ∃c' ∈ V where c' ≈ c* (edit_distance < θ):
8.    MERGE_IDENTITIES(c', c*)
9.  Return c*
```

### 8.2 Identity Merge

When two symbols converge to the same canonical form:

```
Algorithm: MERGE_IDENTITIES(v_old, v_new)

1.  For each edge (u, v_old) ∈ E:
2.    If (u, v_new) ∉ E: CREATE_EDGE(u, v_new, type, w)
3.    Else: w(u, v_new) ← max(w(u, v_new), w(u, v_old))
4.  For each edge (v_old, u) ∈ E:
5.    If (v_new, u) ∉ E: CREATE_EDGE(v_new, u, type, w)
6.    Else: w(v_new, u) ← max(w(v_new, u), w(v_old, u))
7.  v_new.Γ ← max(v_new.Γ, v_old.Γ)
8.  v_new.R ← MERGE_HISTORY(v_new.R, v_old.R)
9.  MARK_DORMANT(v_old)    // Preserve, don't delete
```

---

## 9. Topology Engine

### 9.1 Edge Creation

Edges are created when a semantic relationship is established between two atoms:

$$\text{CREATE\_EDGE}(s, d, \text{type}, w_0)$$

Initial weight: $w_0 = 0.5$ (neutral). Weight evolves via the dynamics in Section 2.3.

### 9.2 Relation Stabilization

Edge stability measures how consistently a relation has been reinforced:

$$\text{stability}(e, t) = 1 - \frac{\sigma(\{w(e, \tau) : \tau \in [t-T_s, t]\})}{\mu(\{w(e, \tau) : \tau \in [t-T_s, t]\}) + \epsilon}$$

where $\sigma$ is standard deviation, $\mu$ is mean, $T_s$ is the stabilization window (default: 50 ticks).

**Stabilization threshold:** $\theta_{\text{stab}} = 0.8$. Edges above this are considered stable and resist decay.

### 9.3 Graph Partitioning

For distributed cognition, the graph is partitioned using spectral partitioning:

$$\text{partition}(G, k) = \text{spectral\_cluster}(L, k)$$

where $L$ is the graph Laplacian. This minimizes inter-partition edges while balancing partition sizes.

---

## 10. Determinism Proof

### 10.1 Theorem

**Theorem.** The DCMA cognitive evolution function $\Phi$ is deterministic: for any state $S$ and input $I$, $\Phi(S, I)$ produces exactly one successor state $S'$.

### 10.2 Proof

We prove that every component of $\Phi$ is deterministic:

**Activation dynamics (Section 3).** The update function is a composition of:
- Rational arithmetic (addition, multiplication, division) — deterministic over $\mathbb{Q}$
- The sigmoid function $\sigma(x) = \frac{1}{1+e^{-x}}$ — deterministic for rational $x$ when computed with fixed precision
- $\max$, $\min$ operations — deterministic

All parameters ($\eta_r, \eta_d, \eta_c, \eta_q, \lambda, H$) are fixed rational constants. The weight normalization uses exact rational division. Therefore, $A(t+1)$ is uniquely determined by $A(t)$.

**Retrieval (Section 4).** Beam search with fixed beam width and deterministic tie-breaking (by node ID lexicographic order) produces a unique result for a given graph state.

**Contradiction dynamics (Section 5).** Pressure update uses only rational arithmetic with fixed parameters. Equilibrium classification uses fixed thresholds.

**Entropy regulation (Section 6).** All operations are triggered by deterministic threshold comparisons on rational values.

**Consolidation (Section 7).** Motif detection uses bounded enumeration with canonical isomorphism testing (deterministic). Crystallization uses fixed thresholds.

**Symbol canonicalization (Section 8).** The pipeline is a sequence of deterministic string operations and table lookups.

**Topology operations (Section 9).** Edge creation, weight updates, and partitioning use deterministic algorithms with fixed parameters.

**Conclusion.** Since every component of $\Phi$ is deterministic, $\Phi$ itself is deterministic. $\blacksquare$

### 10.3 Floating-Point Considerations

For practical implementation, we specify:
- All weights and activations stored as fixed-point rationals with precision $10^{-8}$
- Sigmoid computed via piecewise rational approximation with error $< 10^{-10}$
- No floating-point arithmetic in the core cognitive loop
- External interfaces (APIs, serialization) convert to/from float64 at boundaries

This ensures bit-level reproducibility across platforms and implementations.

---

## 11. Distributed Cognition Protocol

### 11.1 Architecture

Multiple agents share a single cognitive substrate:

```
Agent A  ─┐
Agent B  ─┼──→  Cognitive Memory Protocol  ──→  Distributed Cognitive Graph
Agent C  ─┘                                      (partitioned, replicated)
```

### 11.2 Consistency Model

The substrate uses **eventual consistency with causal ordering**:

- All writes are timestamped with a hybrid logical clock (HLC)
- Conflicting writes to the same edge are resolved by reinforcement accumulation (not last-write-wins)
- Partition healing uses CRDT-style merge for edge weights: $w_{\text{merged}} = \max(w_1, w_2)$

### 11.3 Cross-Agent Continuity

When Agent A stores information, Agent B can retrieve it:

$$\text{retrieve}(B, q) \subseteq G_{\text{shared}}$$

The cognitive substrate is the persistent intelligence layer. Agents are execution shells.

---

## 12. Complexity Analysis

| Operation | Time Complexity | Space Complexity |
|-----------|----------------|-----------------|
| Activation propagation (T iterations) | $O(T \cdot (|V| + |E|))$ | $O(|V|)$ |
| Retrieval (beam search) | $O(d_{\max} \cdot b \cdot d_{\text{avg}} \cdot n_{\max})$ | $O(b \cdot n_{\max})$ |
| Contradiction update | $O(|\mathcal{C}|)$ | $O(|\mathcal{C}|)$ |
| Entropy computation | $O(|V|)$ | $O(d_{\max})$ |
| Motif detection | $O(|V|^{s_{\max}})$ | $O(|V|)$ |
| Crystallization | $O(|V| + |E|)$ | $O(1)$ amortized |
| Symbol canonicalization | $O(L)$ where $L$ = symbol length | $O(1)$ |
| Full consolidation pass | $O(|V|^{s_{\max}} + |E|)$ | $O(|V|)$ |

**Typical parameters:** $|V| \sim 10^5$, $|E| \sim 10^6$, $d_{\text{avg}} \sim 20$, $s_{\max} = 5$, $b = 20$, $T = 80$.

**Full tick (all engines):** $O(|V|^5)$ worst-case (motif detection dominates). In practice, motif detection runs infrequently (every $T_{\text{consolid}}$ ticks), so amortized cost per tick is $O(|V| + |E|)$.

---

## Appendix A — Parameter Reference

| Parameter | Symbol | Default | Description |
|-----------|--------|---------|-------------|
| Activation decay rate | $\eta_d$ | 0.1 | Per-tick decay of node activation |
| Reinforcement rate | $\eta_r$ | 0.05 | Reinforcement energy per event |
| Contradiction suppression | $\eta_c$ | 0.15 | Activation reduction from contradiction |
| Context charge | $\eta_q$ | 0.1 | Query context influence |
| Reinforcement decay | $\lambda$ | 0.9 | Historical reinforcement decay |
| Reinforcement window | $H$ | 10 | Ticks of reinforcement history |
| Convergence threshold | $\epsilon$ | $10^{-6}$ | Activation convergence |
| Max iterations | $T_{\max}$ | 100 | Activation propagation limit |
| Equilibrium threshold | $\theta_{\text{eq}}$ | 0.05 | Contradiction equilibrium |
| Equilibrium window | $T_{\text{eq}}$ | 100 | Ticks for equilibrium check |
| Resolution threshold | $\theta_{\text{resolve}}$ | 0.95 | Contradiction resolution |
| Crystallization threshold | $\theta_{\text{cryst}}$ | 5 | Min motif recurrences |
| Stability threshold | $\theta_{\text{stab}}$ | 0.7 | Min motif stability |
| Entropy lower bound | $H_{\min}$ | 1.5 | Minimum graph entropy |
| Entropy upper bound | $H_{\max}$ | 5.0 | Maximum graph entropy |
| Dead node threshold | $\theta_{\text{dead}}$ | $10^{-6}$ | Activation below which node is dead |
| Dead node window | $T_{\text{dead}}$ | 10000 | Ticks before dead elimination |
| Spontaneous activation | $p_{\text{spont}}$ | $10^{-4}$ | Per-node per-tick probability |
| Max motif size | $s_{\max}$ | 5 | Maximum motif subgraph size |
| Max abstraction depth | — | 5 | Maximum abstraction hierarchy levels |
| Beam width | $b$ | 20 | Retrieval beam search width |
| Max retrieval depth | $d_{\max}$ | 4 | Maximum retrieval traversal hops |
| Max subgraph size | $n_{\max}$ | 500 | Maximum retrieval subgraph nodes |

---

## Appendix B — Notation Reference

| Symbol | Meaning |
|--------|---------|
| $G(t)$ | Cognitive graph at time $t$ |
| $V(t)$ | Node set at time $t$ |
| $E(t)$ | Edge set at time $t$ |
| $W(t)$ | Edge weight function at time $t$ |
| $A(t)$ | Activation vector at time $t$ |
| $S(t)$ | Full cognitive state at time $t$ |
| $\Phi$ | Cognitive evolution function |
| $\mathcal{C}(t)$ | Contradiction structure at time $t$ |
| $\Gamma(v,t)$ | Salience gravity of node $v$ at time $t$ |
| $\mathcal{N}(i)$ | Inbound neighbors of node $i$ |
| $H(G)$ | Shannon entropy of graph $G$ |
| $\mathbb{Q}$ | Set of rational numbers |
| $\sigma$ | Sigmoid normalization function |

---

*Whilmar M. Bitoco — IICT, Aces Tagum College Inc.*
*GitHub: github.com/whilmarbitoco*
