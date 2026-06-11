-- DCMA PostgreSQL Schema
-- Run: psql -h llm-ciel-memory-qk6cz9 -p 5432 -U dcma -d dcma -f schema.sql

-- ─── Atoms (Cognitive Atoms) ────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_atoms (
    id              UUID PRIMARY KEY,
    atom_type       TEXT NOT NULL CHECK (atom_type IN ('Concept','Event','Entity','Procedure','Abstraction')),
    canonical       TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'Active' CHECK (state IN ('Active','Dormant','Frozen','Consolidated')),
    attributes      JSONB NOT NULL DEFAULT '{}',
    salience        NUMERIC(10,8) NOT NULL DEFAULT 0,
    confidence      NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 1),
    origin          TEXT NOT NULL DEFAULT '',
    t_created       BIGINT NOT NULL DEFAULT 0,
    t_last_accessed BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dcma_atoms_canonical ON dcma_atoms (canonical);
CREATE INDEX IF NOT EXISTS idx_dcma_atoms_type ON dcma_atoms (atom_type);
CREATE INDEX IF NOT EXISTS idx_dcma_atoms_state ON dcma_atoms (state);
CREATE INDEX IF NOT EXISTS idx_dcma_atoms_salience ON dcma_atoms (salience DESC);
CREATE INDEX IF NOT EXISTS idx_dcma_atoms_canonical_trgm ON dcma_atoms USING gin (canonical gin_trgm_ops);

-- ─── Relations (Edges) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_relations (
    id              BIGSERIAL PRIMARY KEY,
    src_atom_id     UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    dst_atom_id     UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    relation_type   TEXT NOT NULL,
    weight          NUMERIC(10,8) NOT NULL DEFAULT 0.5 CHECK (weight >= 0 AND weight <= 1),
    stability       NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (stability >= 0 AND stability <= 1),
    confidence      NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 1),
    t_created       BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(src_atom_id, dst_atom_id, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_dcma_rel_src ON dcma_relations (src_atom_id);
CREATE INDEX IF NOT EXISTS idx_dcma_rel_dst ON dcma_relations (dst_atom_id);
CREATE INDEX IF NOT EXISTS idx_dcma_rel_type ON dcma_relations (relation_type);

-- ─── Contradictions ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_contradictions (
    atom_a_id       UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    atom_b_id       UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    pressure        NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (pressure >= 0 AND pressure <= 1),
    equilibrium     TEXT NOT NULL DEFAULT 'StableEquilibrium'
                    CHECK (equilibrium IN ('StableEquilibrium','DominantEquilibrium','Oscillating','Resolved')),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (atom_a_id, atom_b_id)
);

-- ─── Tick Log ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_ticks (
    tick_number     BIGINT PRIMARY KEY,
    state_hash      TEXT NOT NULL,
    atom_count      INT NOT NULL,
    relation_count  INT NOT NULL,
    entropy         NUMERIC(10,8),
    convergence_iters INT,
    duration_ms     INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dcma_ticks_created ON dcma_ticks (created_at);

-- ─── State Snapshots (periodic) ────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_snapshots (
    id              BIGSERIAL PRIMARY KEY,
    tick_number     BIGINT NOT NULL,
    state_data      BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dcma_snapshots_tick ON dcma_snapshots (tick_number);

-- ─── Reinforcement History ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_reinforcements (
    id              BIGSERIAL PRIMARY KEY,
    atom_id         UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    tick            BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dcma_reinf_atom ON dcma_reinforcements (atom_id, tick);

-- ─── Activation History ────────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_activations (
    id              BIGSERIAL PRIMARY KEY,
    atom_id         UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    tick            BIGINT NOT NULL,
    activation      NUMERIC(10,8) NOT NULL CHECK (activation >= 0 AND activation <= 1),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_dcma_act_atom ON dcma_activations (atom_id, tick);

-- ─── Configuration ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS dcma_config (
    key             TEXT PRIMARY KEY,
    value           JSONB NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
