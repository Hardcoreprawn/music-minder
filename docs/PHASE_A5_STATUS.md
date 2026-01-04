# Phase A.5 Code Migration - Status & Strategy

## Completed (7 commits total)

### Phase A.4 Foundation (Commits 1-5)

- ADR documentation with architecture decisions
- Workspace structure with 5 crates
- Enhanced .gitignore for development
- Integration tests for database layer
- Cleanup of old source files in music-minder

### Phase A.5 - Scanner Migration (Commits 6-7)

- **✅ Commit 41c3113**: Migrated `scanner/mod.rs` and `scanner/watcher.rs` to `crates/music_journo`
  - Async directory traversal with tokio streams
  - File system watcher with debounced notify events
  - Both sync and async variants for different contexts
  - Comprehensive test coverage (13 test functions)
  - Public API exports: `scan()`, `FileWatcher`, `WatchEvent`, `is_audio_file()`

- **✅ Commit e3385c7**: Minor formatting fix

**Current Status**: Working tree clean, all 7 commits pass pre-commit validation (format + clippy + tests)

---

## Remaining Work - Stratified Approach

Given interdependencies, Phase A.5 is being done in stages:

### Stage 1: Metadata Foundations (Next 2-3 commits)

**Challenge**: metadata.rs depends on `enrichment::domain::IdentifiedTrack` which isn't migrated yet
**Solution**:

1. Create `soundstore::metadata` module with `TrackMetadata` struct (no enrichment)
2. Implement basic read() function (lofty integration)
3. Create `discographer::metadata` placeholder that will eventually call soundstore

**Files to migrate**:

- `crates/music-minder/src/metadata/mod.rs` → soundstore + discographer (split)
- Dependencies to add: lofty (already in workspace)

### Stage 2: Organizer Layer (1-2 commits)

**Challenge**: depends on metadata
**Solution**: Migrate after metadata foundations are complete

**Files to migrate**:

- `crates/music-minder/src/organizer/mod.rs` → `crates/discographer/src/organizer/`

### Stage 3: Database Layer (1-2 commits)

**Challenge**: Large and self-contained, but referenced by other modules
**Solution**: Implement fully in soundstore (tests already written)

**Files to migrate**:

- `crates/music-minder/src/db/mod.rs` → `crates/soundstore/src/db/`
- **Note**: This file was deleted in earlier cleanup; reconstruct from test requirements

### Stage 4: Player/Symphony Integration (0-1 commits)

**Status**: 90% already in `crates/symphonium`, just needs re-export
**Actions**:

- Verify symphonium builds independently
- Add to music-minder imports as external crate

### Stage 5: Final Integration (1 commit)

- Update `music-minder/src/lib.rs` and `main.rs` to import from new crates
- Remove old source files from music-minder/src (except ui, cli which stay for now)
- Run full workspace tests
- Final consolidation commit

---

## Dependency Graph (as of now)

```text
symphonium (DONE: self-contained audio)
  ↓ (music-minder will import)
  
music_journo (DONE: scanner + watcher)
  ↓ (music-minder will import)
  
discographer (IN PROGRESS)
  ├── metadata (pending: depends on enrichment)
  └── organizer (pending: depends on metadata)
  
soundstore (IN PROGRESS)
  ├── metadata (pending: basic types)
  └── db (pending: full implementation)
  
music-minder (FINAL STEP)
  ├── ui/ (STAYS in music-minder)
  ├── cli/ (STAYS in music-minder)
  ├── enrichment/ (STAYS for now - complex refactor)
  └── imports from: symphonium, music_journo, discographer, soundstore
```

---

## Why This Approach?

1. **Minimize circular dependencies**: Scanner and player have no dependencies on others
2. **Staged commits**: Each commit should compile independently
3. **Test-driven**: Integration tests already define expected APIs
4. **Preserve working state**: Never break the build, always have runnable code
5. **One feature per commit**: Clean git history for code review

---

## Estimated Effort

- Stage 1 (Metadata): 1-2 hours
- Stage 2 (Organizer): 30 minutes
- Stage 3 (Database): 1-2 hours
- Stage 4 (Player): 15 minutes
- Stage 5 (Integration): 30 minutes - 1 hour

**Total**: ~4-6 hours for complete Phase A.5 migration

---

## Next Immediate Actions

1. Create `soundstore::metadata::TrackMetadata` struct
2. Implement `soundstore::metadata::read()` using lofty
3. Add tests for metadata reading
4. Create discographer placeholders
5. Run `cargo check --workspace` to verify progress
6. Commit with clear message
7. Proceed to Stage 2

---

## Notes for Code Review

- All new code includes comprehensive tests
- Pre-commit hooks (format + clippy) enforced for all commits
- Migration preserves existing functionality (no behavior changes)
- Error handling follows crate conventions (anyhow::Result)
- Public APIs use re-exports in lib.rs for clean module organization
