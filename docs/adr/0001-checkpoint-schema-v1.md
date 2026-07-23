# ADR 0001: Checkpoint Schema v1 — Content-Addressed Manifest

## Status

Accepted

## Date

2026-07-22

## Context

`decon` runs a long, expensive pipeline against code repositories (local trees or
GitHub URLs). A full monorepo tutorial can require dozens of LLM map/reduce
calls, so every expensive stage must be **idempotent and resumable**.

The Python reference implementation stores the entire tutorial generation state
in a single JSON checkpoint file. That works for small repositories, but it
does not scale:

- File bodies are duplicated inside the checkpoint, inflating it to hundreds of
  megabytes for large trees.
- Writing a new checkpoint after every stage becomes an I/O bottleneck.
- Resuming requires loading the entire blob into memory even when only a subset
  of files is needed for the next stage.

We need a durable, versioned checkpoint format that survives interrupts,
supports partial resume, and remains bounded in size as the analyzed repository
grows.

## Decision

Checkpoint v1 is a **small, versioned JSON metadata file** plus a **sidecar file
bundle** for the actual file bodies.

### Metadata file (`checkpoint.json`)

A stable JSON object that contains everything needed to resume the pipeline:

```json
{
  "version": 1,
  "completed_stages": ["fetch", "identify"],
  "config_hash": "sha256:...",
  "config": { "...": "..." },
  "files_manifest": "files.ndjson.gz",
  "abstractions": [...],
  "relationships": {...},
  "metadata": {
    "created_at": "...",
    "updated_at": "...",
    "source_revision": "git-sha-or-url"
  }
}
```

Key rules:

- `version` is required and monotonically incremented for breaking schema changes.
- `config_hash` is a SHA-256 of the canonicalized `config` object.
- `files_manifest` is a path to the sidecar, relative to the checkpoint
  directory.
- The JSON checkpoint stores **file paths and SHA-256 hashes**, not full bodies.

### File bundle (`files.ndjson.gz`)

File bodies live in a gzipped newline-delimited JSON file with one record per
line:

```json
{"path": "src/main.rs", "sha256": "...", "content": "..."}
{"path": "Cargo.toml", "sha256": "...", "content": "..."}
```

Rules:

- Each record is self-contained and content-addressed by `sha256`.
- The bundle is append-friendly and streamable during crawl.
- Records can be rebuilt from the source repository on resume if the manifest is
  missing but the same crawl filters still apply.

### Resume behavior

1. Load `checkpoint.json` and validate `version`.
2. Recompute `config_hash` from `config` and warn if it differs from the current
   run config.
3. Open `files_manifest` and build an in-memory index `path -> sha256 -> content`.
4. Re-crawl only if `config_hash` changed or the manifest is corrupt/missing.

## Alternatives Considered

### Monolithic JSON checkpoint

- **Pros**: Single file, trivial to read and write.
- **Cons**: Quadratic memory growth with repo size; hard to stream; Python
  reference already hits this wall.
- **Rejected**: Does not meet the scalability goal stated in the product spec.

### Pure content-addressed blob store (one file per blob)

- **Pros**: Deduplication is automatic; easy to verify integrity.
- **Cons**: Large number of small files; filesystem overhead; harder to ship
  and archive as a unit.
- **Rejected**: More complex than needed for v1. `ndjson.gz` keeps all bodies in
  one compressed stream that is easy to version and archive.

### Re-crawl on every resume

- **Pros**: No separate manifest to manage.
- **Cons**: Re-reads the entire repository, which is slow and may race with
  working-tree changes.
- **Rejected**: Resume should be deterministic and fast; the manifest is the
  source of truth for the snapshot that produced the checkpoint.

## Consequences

- **Positive**: Checkpoint JSON stays small and fast to parse regardless of
  repository size.
- **Positive**: The manifest is streamable, so crawl can append bodies as it
  discovers them without holding the whole tree in memory.
- **Positive**: SHA-256 hashes let the pipeline detect unchanged files and skip
  re-processing during incremental runs.
- **Negative**: Two files to manage instead of one; tooling must copy/move both
  together.
- **Negative**: A corrupt `files.ndjson.gz` requires re-crawl, which may be
  expensive; future iterations may add a manifest checksum.

## Related Documents

- `docs/move-to-rust.md` §2.2 and §4.4 — staged pipeline and proposed checkpoint
  shape.
- `docs/best-practices.md` — tutorial quality rules that the checkpoint stages
  must preserve.
- `crates/decon-pipeline/src/lib.rs` — placeholder reference to this ADR.
