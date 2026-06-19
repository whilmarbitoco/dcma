# DCMA Benchmark Report

**Date:** June 19, 2026
**Hardware:** Oracle Cloud (Linux 6.8.0-1054-oracle)
**Rust:** 1.85+ (release mode)
**Criterion:** 0.5.1 (100 samples, 3s warmup, 5s measurement)

---

## Ingestion

| Atoms | Time | Rate |
|---|---|---|
| 1,000 | 522 us | 1.9M atoms/sec |
| 10,000 | 5.84 ms | 1.7M atoms/sec |
| 50,000 | 35.37 ms | 1.4M atoms/sec |
| 100,000 | 72.07 ms | 1.4M atoms/sec |

With 50% relations:

| Atoms + Relations | Time | Rate |
|---|---|---|
| 1,000 + 500 | 592 us | 1.7M items/sec |
| 10,000 + 5,000 | 6.82 ms | 1.5M items/sec |
| 50,000 + 25,000 | 41.11 ms | 1.2M items/sec |

---

## Cognitive Tick

| Graph Size | Time | Per-atom |
|---|---|---|
| 100 atoms, 50 relations | 2.46 ms | 24.6 us/atom |
| 1,000 atoms, 500 relations | 22.35 ms | 22.4 us/atom |
| 5,000 atoms, 2,500 relations | 111.79 ms | 22.4 us/atom |
| 10,000 atoms, 5,000 relations | 231.20 ms | 23.1 us/atom |

Tick time scales linearly with graph size (~23us/atom).

---

## Retrieval

| Graph Size | Time |
|---|---|
| 1,000 atoms | 21.64 ms |
| 5,000 atoms | 111.86 ms |
| 10,000 atoms | 231.14 ms |

---

## Contradiction Detection

| Graph Size | Time |
|---|---|
| 100 atoms, 50 contradictions | 1.43 ms |
| 500 atoms, 250 contradictions | 7.15 ms |
| 1,000 atoms, 500 contradictions | 14.40 ms |

---

## Summary

| Metric | Value |
|---|---|
| Max ingestion rate | 1.9M atoms/sec |
| Tick throughput (10K graph) | ~43 ticks/sec |
| Retrieval latency (10K graph) | ~231 ms |
| Contradiction detection (1K) | ~14 ms |
| Total tests | 225 passing |
| Integration tests | 26 new tests across 7 files |
