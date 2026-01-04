# Architecture Decision Record: Monorepo & Ecosystem Strategy

**Date:** January 2026  
**Status:** PROPOSED  
**Context:** Music Minder is reaching complexity (~12.5k LOC) where architectural refactoring will improve maintainability and reusability.

---

## The Problem

Music Minder is a monolithic Rust application mixing:

- **Audio pipeline** (decode, resample, output) — could be reusable
- **Metadata enrichment** (fingerprinting, API clients, matching) — could be standalone service
- **File management** (scanning, organizing) — could be CLI tool
- **Database layer** (SQLite, migrations, entities) — could support multiple backends
- **GUI** (Iced, state management, views) — tightly coupled to everything above

As features grow, the monolith becomes harder to test, optimize, and reuse. This ADR defines the extraction strategy.

---

## Decision 1: Monorepo Workspace Structure

### Option A: Monorepo (Workspace with local crates)

#### Chosen: YES

**Rationale:**

1. **Unified versioning** — All crates release together as v0.2.0, v0.3.0, etc.
2. **Shared development** — Changes to audio API update GUI simultaneously
3. **Easy migration** — Can publish individual crates to crates.io later without major refactoring
4. **Single test suite** — `cargo test` runs all crates, catches cross-crate issues
5. **Simpler build** — One Cargo.lock, one build cache, easier CI/CD
6. **Lower friction** — Team doesn't need multiple repos initially

**Timeline:**

- **Phase 0 (Week 1):** Create workspace Cargo.toml, move existing code to `crates/music-minder/`
- **Phase 1 (Week 2-3):** Extract symphonium, discography, librarian, soundstore as local crates
- **Phase 2 (Week 4+):** Publish stable crates to crates.io if demand exists

**Future Migration Path:**
If one crate (e.g., symphonium) becomes popular, we can:

1. Move to separate repository
2. Copy code to new repo, rebase history
3. Update main workspace to import from crates.io
4. Minimal disruption because interfaces are already stable

### Option B: Separate Repositories

Not chosen at this time. Adds complexity early without clear benefit.

---

## Decision 2: New Crate Names

**Rationale:** Generic `music-minder-<suffix>` names don't convey purpose or personality. Named crates are more memorable and can stand alone.

### Crate Naming Strategy

| Purpose | New Name | Rationale | Publishable |
| ------- | -------- | --------- | ----------- |
| Audio decode/resample/output pipeline | **symphonium** | Musical metaphor; evokes Symphonia library; publishable standalone | ✅ Yes |
| Metadata enrichment (fingerprint, identify, enrich) | **discography** | Standard music industry term; clearly implies metadata work | ✅ Yes |
| File scanning, organizing, metadata I/O | **librarian** | Evokes library management; clear purpose | ✅ Yes |
| Database schema, migrations, entities | **soundstore** | Implies persistent storage of sound data | ⏳ Maybe (after schema stabilizes) |

**Workspace structure:**

```text
music-minder/                          # Workspace root
├── Cargo.toml                         # Workspace definition
├── docs/
│   ├── ROADMAP.md
│   ├── ADR_ARCHITECTURE_EXTRACTION.md (this file)
│   └── ...
├── crates/
│   ├── symphonium/                    # Audio pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── discography/                   # Metadata enrichment
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── librarian/                     # File management
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── soundstore/                    # Database + schema
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── migrations/
│   └── music-minder/                  # Main app (GUI + CLI)
│       ├── Cargo.toml
│       └── src/
└── migrations/                         # Root-level migrations (auto-included)
```

---

## Decision 3: Ecosystem Crate Adoption

### Current Custom Implementations vs Ecosystem Alternatives

**Audio Stack** — Keep all custom components

| Component | Current | Ecosystem Alternative | Decision |
| --------- | ------- | -------------------- | -------- |
| Format decode | Symphonia | `metaflac`, `mp3_metadata` | **Keep Symphonia** (supports all formats, pure Rust, no FFI) |
| Audio output | CPAL | — | **Keep CPAL** (only cross-platform solution) |
| Resampling | Rubato | `sinc`, `polyphonic` | **Keep Rubato** (highest quality, lock-free ring buffer) |
| Ring buffer | rtrb | — | **Keep rtrb** (lock-free, real-time safe) |
| FFT | realfft + rustfft | `fftw` (FFI) | **Keep realfft** (pure Rust, no FFI, real-time safe) |

**Decision:** Audio stack is optimal. No changes.

---

**File Management** — Adopt camino for UTF-8 paths

| Component | Current | Ecosystem Alternative | Decision |
| --------- | ------- | -------------------- | -------- |
| Paths | `std::path::Path` | **`camino::Utf8Path`** | **Adopt camino** |
| File discovery | `walkdir` | — | **Keep walkdir** (excellent, simple) |
| Metadata reading | `lofty` | `metaflac`, `metaflac-rs`, `id3` | **Keep lofty** (unified tag interface) |

**Rationale for camino:** Music filenames are UTF-8 (FLAC tags, ID3 tags, display names). Storing as `Utf8Path` eliminates `to_string_lossy()` conversions and type confusion. Benefits:

- All paths are valid UTF-8 (enforced by type system)
- Can safely display in UI without validation
- Matches database string columns exactly
- No `OsStr`/`OsString` confusion in APIs

**Action:** Add `camino = "1.1"` to `librarian` crate; use `Utf8Path` and `Utf8PathBuf` throughout scanner/organizer.

---

**Database** — Keep SQLx + custom repository pattern (don't adopt sea-orm)

| Component | Current | Ecosystem Alternative | Decision |
| --------- | ------- | --------------------- | -------- |
| SQL queries | SQLx (async, compile-time checked) | **sea-orm** (ORM) | **Keep SQLx** |
| Query builder | Hand-written | `sqlx::QueryBuilder` | **Use QueryBuilder** for dynamic queries |
| Schema migrations | Custom SQL in `/migrations` | `sea-orm-cli` | **Keep current** (migrations are stable, few changes expected) |

**Rationale for keeping SQLx:**

- Compile-time query validation catches errors early
- No ORM overhead for a simple schema
- Direct control over performance-critical paths
- Can add repository pattern for abstraction without ORM

**Rationale against sea-orm:**

- ORM adds overhead for a ~10-table schema
- Would need to rewrite existing queries
- Loses compile-time verification
- Repository pattern gives us most of the abstraction benefits

---

**Testing & Benchmarking** — Adopt criterion for performance auditing

| Component | Current | Ecosystem Alternative | Decision |
| --------- | ------- | -------------------- | -------- |
| Unit tests | `#[test]` | — | **Keep current** |
| Property tests | `proptest` | — | **Keep proptest** |
| Benchmarking | None | **`criterion`** | **Adopt criterion** |

**Action:** Create `benches/` directory in symphonium:

```rust
// benches/decode.rs
#[macro_use]
extern crate criterion;
use criterion::Criterion;
use symphonium::Decoder;

fn bench_decode_mp3(c: &mut Criterion) {
    c.bench_function("decode_mp3_128kb_3min", |b| {
        b.iter(|| Decoder::new(test_file).and_then(|d| d.decode_all()))
    });
}

criterion_group!(benches, bench_decode_mp3);
criterion_main!(benches);
```

**Benefits:**

- Track audio decode performance across releases
- Identify regressions in FFT, resampling
- Publish benchmarks with crate (crates.io consumers see performance)
- `criterion compare` for before/after optimization

---

**Async patterns** — Keep crossbeam channels, consider bounded channels for backpressure

| Component | Current | Ecosystem Alternative | Decision |
| --------- | ------- | -------------------- | -------- |
| Lock-free queues | `crossbeam::queue::ArrayQueue` | `tokio::sync::mpsc` (bounded) | **Hybrid approach** |
| Audio command queue | Unbounded `crossbeam::channel` | — | **Keep unbounded** (low volume, needs predictability) |
| Library scan batches | Unbounded `crossbeam::channel` | `tokio::sync::mpsc::bounded` | **Consider bounded** (high volume, apply backpressure) |

**Decision:** Current crossbeam usage is fine. If scan performance becomes bottleneck, switch scanner channel to bounded with backpressure. No changes needed now.

---

## Decision 4: Extraction Order

**Leaves-first dependency graph:**

```text
soundstore (no deps)
    ↑
    ├── symphonium (needs soundstore for optional query support)
    ├── librarian (depends on soundstore for writes)
    └── discography (depends on soundstore for query layer)
        
music-minder (main GUI + CLI, depends on all above)
```

**Extraction sequence:**

1. **Week 1:** Create workspace, establish soundstore (schema + repository traits)
2. **Week 2:** Extract symphonium (audio pipeline, no DB deps)
3. **Week 3:** Extract librarian (file management, depends on soundstore)
4. **Week 4:** Extract discography (enrichment, depends on soundstore)
5. **Week 5+:** Refactor music-minder to use extracted crates

**Why this order:**

- Extract no-dependency crates first (soundstore can stand alone)
- Build up dependencies gradually
- Each extraction is testable before next begins
- Main app refactoring is final, highest-confidence step

---

## Decision 5: Publishing Strategy

### Phase 1: Private/Workspace Crates (v0.2.0 - v0.5.0)

All crates remain in workspace, published privately:

- No crates.io publishing yet
- Develop for 6+ months, stabilize APIs
- Gather feedback from early users
- Fix breaking changes while crates are young

### Phase 2: Public Crates (v0.6.0+, conditional)

Only publish crates that meet criteria:

- ✅ **API stable** — No breaking changes for 2 releases
- ✅ **Well-tested** — 80%+ coverage, real-world usage in Music Minder
- ✅ **Documented** — rustdoc, examples, use cases
- ✅ **Unique value** — Solves a problem not solved elsewhere

**Likely candidates:**

- **symphonium** — Combines Symphonia + CPAL + Rubato; valuable for music apps/streaming
- **discography** — Metadata enrichment pipeline; useful for music organizers
- **librarian** — File scanning/organizing with UTF-8 paths; useful for backup tools
- **soundstore** — Less likely (too specific to Music Minder's schema)

**Not publishing:**

- music-minder (main app, too specific)

---

## Decision 6: Continuous Integration

### Testing Strategy

```bash
# Test all crates in workspace
cargo test --workspace

# Test each crate individually (catches missing dependencies)
cargo test -p symphonium
cargo test -p discography
cargo test -p librarian
cargo test -p soundstore
cargo test -p music-minder

# Bench symphonium audio pipeline
cargo bench -p symphonium

# Lint all crates
cargo clippy --workspace -- -D warnings
```

### Documentation

Each crate should have:

- `README.md` at crate root (describes purpose, usage examples)
- Inline rustdoc (visible on docs.rs when published)
- Examples in `examples/` directory
- Changelog in `CHANGELOG.md`

---

## Decision 7: API Boundaries (What to Extract)

### symphonium (Audio Pipeline)

**Public API:**

```rust
pub use symphonium::*;

// Core types
pub struct AudioPipeline { /* ... */ }
pub struct AudioSharedState { /* ... */ }
pub enum PlayerEvent { Play, Pause, Stop, Error(String) }
pub enum PlayerCommand { Play(Path), Stop, Seek(Duration) }

// Trait for custom audio callbacks
pub trait AudioOutput: Send + Sync {
    fn write_samples(&mut self, samples: &[f32]) -> Result<()>;
}
```

**Private (stays in music-minder):**

- Queue management (app-specific concern)
- Visualization binding (UI concern)
- Media control integration (OS integration)

---

### discography (Metadata Enrichment)

**Public API:**

```rust
pub use discography::*;

pub struct EnrichmentService { /* ... */ }
pub struct TrackIdentification { /* ... */ }
pub enum ApiClient { AcoustID, MusicBrainz, CoverArt }

impl EnrichmentService {
    pub async fn identify(&self, file: &Path) -> Result<TrackIdentification>;
    pub async fn enrich(&self, track: &Track) -> Result<EnrichedMetadata>;
}
```

**Private (stays in music-minder):**

- UI for enrichment results
- Database integration (writing to DB)

---

### librarian (File Management)

**Public API:**

```rust
pub use librarian::*;

pub struct Scanner { /* ... */ }
pub struct Organizer { /* ... */ }
pub struct MetadataReader { /* ... */ }

impl Scanner {
    pub fn scan(path: &Utf8Path) -> Result<impl Iterator<Item = TrackMetadata>>;
}

impl Organizer {
    pub fn organize(src: &Utf8Path, pattern: &str, dry_run: bool) -> Result<Vec<Operation>>;
}
```

**Private (stays in music-minder):**

- Database integration
- Watcher/subscription integration

---

### soundstore (Database)

**Public API:**

```rust
pub use soundstore::*;

pub struct Migration { /* ... */ }
pub trait TrackRepository: Send + Sync {
    async fn get_track(&self, id: i64) -> Result<Track>;
    async fn insert_track(&self, track: Track) -> Result<()>;
}

pub struct SqliteRepository { /* ... */ }
```

---

## Success Criteria

✅ **After extraction, the project should:**

1. Have clearly isolated concerns
   - Audio pipeline testable without UI
   - Enrichment testable without database
   - File management testable without GUI

2. Support new architectures
   - Headless audio server (just symphonium)
   - Metadata batch processor (just discography)
   - File organizer CLI (librarian + soundstore)

3. Maintain performance
   - No runtime overhead from module boundaries
   - Lock-free audio path unchanged
   - Database queries unchanged

4. Enable future evolution
   - Can add PostgreSQL support (soundstore) without touching audio
   - Can optimize scanning (librarian) without touching enrichment
   - Can add new audio codecs (symphonium) without touching GUI

---

## Appendix: Reference Implementation Timeline

**Month 1 (Jan 2026):**

- Week 1: Workspace setup, create soundstore skeleton
- Week 2: Extract symphonium (core audio types)
- Week 3: Extract librarian (file types, path handling)
- Week 4: Extract discography (API client types)

**Month 2 (Feb 2026):**

- Week 5-6: Refactor music-minder to use extracted crates
- Week 7-8: Test, fix issues, performance validation

**Month 3+ (Mar 2026):**

- Stabilize APIs based on feedback
- Add criterion benchmarks
- Polish documentation
- Publish to crates.io (if justified)
