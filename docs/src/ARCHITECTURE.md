# TF-IDF System: Master Architecture Document

**Version**: 1.0.0  
**Date**: 2025-12-01  
**Architect**: Me
**Status**: *Conception*
**Repository**: Forgejo (self-hosted)

---

## Executive Summary

A capability-driven system for document analysis, similarity detection, and knowledge graph construction using TF-IDF and category-theoretic principles. Designed for single-developer cognitive load optimization through:

1. **Composition**: Small, composable modules (functors)
2. **Context locality**: Information pulled toward user based on task
3. **Feedback loops**: Use tool output to improve tool (dogfooding)
4. **Hypergraph emergence**: Local term relations reveal global structure
5. **Multi-corpus linking**: Notes ↔ Transcripts ↔ Wiki

**Quick Win**: Working rust-script for immediate similarity queries  
**Long-term Goal**: Non-AI semantic embeddings via topological analysis

---

## Document Structure

This architecture is organized by viewpoints for easy navigation:

1. **Capability View** — What the system can do
2. **Operational View** — How users interact with it
3. **Systems View** — Component architecture
4. **Services View** — APIs and interfaces
5. **Data View** — Information flow and storage
6. **Standards View** — Technology choices and rationale
7. **Project View** — Build, test, release process
8. **Theoretical View** — Category theory foundations

---

## 1. CAPABILITY VIEW

> **STATUS**: *ROUGH SKETCH*

### 1.1 Capability Hierarchy

> **Issue**: Need natural mapping of capabilities to phase
> - Need to refer to architecture notes
> - Look at graph drawing notes

```d2 «Capability CV-x Taxonomy»
C1: DOCUMENT ANALYSIS
C1: { SIMILAR DOCS; TERM EXTRACTION }
C2: CROSS-CORPUS 

```

```d2 «CAPABILITY C1»
C1: Document Analysis Capability
    1: Similarity Detection
    2: Term Extraction
    3: Topic Model
    4: Clustering
}
```

```d2 «CAPABILITY C2»
C2: Corpus Linking Capability {
    1: Cross-corpus similarity
    2: Automatic backlink generation
    3: Concept graph construction
}
```

```
C3: Hypergraph Construction {
    1: Term co-occurrence graphs
    2: Document relationship graphs
    3: Meta-relation discovery
}
```

```
C4: Query & Retrieval {
    1: Terminal interface (Phase 1)
    2: Library API (Phase 1)
    3: Web interface (Phase 3)
    4: Neovim integration (Phase 3)
}
```

```
C5: Visualization {
    1: D2 diagram generation
    2: Graph export (Graphviz, Cytoscape)
    3: Interactive exploration (Phase 3)
}
```

```
# Stages - need to parameteris color by phase
# maybe use class
PHASES: "Development Phase" {
P1: "v0.1.0" { style.fill: "#aaffaa"};
P2: "v0.2.0"
    P3: "v0.3.0"
    P4: "v0.4.0"
}


# STAGE.2 <- CAP-1.3
# STAGE-2 <- CAP-1.4
```

### 1.2 Capability-Resource Matrix

| Capability | Required Resources | Status |
|------------|-------------------|--------|
| C1.1 (Similarity) | Tokenizer, Index, Scorer | ✅ Phase 1 |
| C1.2 (Terms) | Tokenizer, Index | ✅ Phase 1 |
| C2.1 (Cross-corpus) | Multi-index manager | 🔄 Phase 2 |
| C3.1 (Co-occurrence) | Graph builder, Matrix ops | 🔄 Phase 2 |
| C4.3 (Web) | HTTP server, Templates | ⏳ Phase 3 |
| C5.1 (D2) | Graph serializer | 🔄 Phase 2 |

### 1.3 Phase Roadmap

**Phase 1: Quick Win (Week 1-2)**
- ✅ Rust-script for immediate use
- ✅ Core library (tokenizer, index, query)
- ✅ CLI with clap
- ⏳ Basic tests

**Phase 2: Hypergraph (Week 3-6)**
- Term co-occurrence matrix
- Graph construction from similarities
- D2 diagram generation
- Cross-corpus linking
- Topic modeling basics

**Phase 3: Ecosystem (Week 7-12)**
- Web interface
- Neovim plugin
- API server (HTTP/WebSocket)
- Advanced clustering
- Visualization tools

---

## 2. OPERATIONAL VIEW

### 2.1 User Personas & Workflows

**Persona 1: Developer (Me) — Research Notes**
```bash
# Find related notes
tfidf similar notes/compilers.md notes/ --corpus wiki

# Generate backlinks
tfidf links notes/compilers.md --target-corpus wiki/ --top 5

# Update note with discovered links
tfidf enrich notes/compilers.md --append-links
```

**Persona 2: Developer (Me) — Code Documentation**
```bash
# Which notes document this code?
tfidf cross-corpus src/parser.rs notes/ --ext md

# Generate documentation stubs
tfidf gen-doc src/parser.rs --template notes/templates/code-doc.md
```

**Persona 3: Developer (Me) — Transcript Analysis**
```bash
# Find wiki pages related to meeting transcript
tfidf similar transcripts/2025-12-01-standup.txt wiki/ --ext md

# Extract key topics
tfidf topics transcripts/ --top 20 --cluster
```

### 2.2 Integration Points

**Shell Integration**
- XDG config: `~/.config/tfidf/config.toml`
- XDG data: `~/.local/share/tfidf/indices/`
- XDG cache: `~/.cache/tfidf/`
- Env vars: `TFIDF_CORPUS_ROOT`, `TFIDF_STOPWORDS`
- Piping: `cat file.md | tfidf similar - corpus/`

**Neovim Integration (Future)**
```vim
:TfidfSimilar          " Find similar to current buffer
:TfidfLinks            " Insert backlinks at cursor
:TfidfTopics           " Show topic distribution
```

**API Integration (Future)**
```rust
// Library usage
let index = tfidf::Index::build(corpus)?;
let similar = index.similar_to("file.md", 10)?;
```

### 2.3 Configuration Hierarchy

```
1. Command-line args (highest priority)
2. Environment variables
3. Project config (.tfidf.toml in PWD)
4. User config (~/.config/tfidf/config.toml)
5. System defaults (compiled-in)
```

---

## 3. SYSTEMS VIEW

### 3.1 Component Architecture

```d2
direction: right

user: User {
  shape: person
}

cli: CLI {
  shape: rectangle
  clap: Clap Parser
  commands: Commands
  output: Formatters
}

lib: Library Core {
  shape: rectangle
  
  corpus: Corpus {
    discover: discover()
    filter: filter()
  }
  
  tokenizer: Tokenizer {
    tokenize: tokenize()
    stopwords: load_stopwords()
  }
  
  index: Index {
    build: build()
    save: save()
    load: load()
  }
  
  query: Query {
    similar: similar_to()
    terms: top_terms()
    links: find_links()
  }
  
  graph: Graph Builder {
    cooccur: co_occurrence()
    hypergraph: build_hypergraph()
  }
}

storage: Storage {
  shape: cylinder
  indices: Index Files
  cache: Query Cache
  config: Config Files
}

viz: Visualization {
  shape: rectangle
  d2: D2 Generator
  dot: Graphviz
  json: JSON Export
}

user -> cli: Commands
cli -> lib: Library Calls
lib -> storage: Read/Write
lib -> viz: Generate Diagrams
```

### 3.2 Module Dependency Graph

```
Level 0 (No dependencies):
  - types (Document, Term, Score)
  - error (Error types)

Level 1 (Foundation):
  - tokenizer → types, error
  - hasher → types
  - glob → error

Level 2 (Core):
  - corpus → tokenizer, glob, types
  - stopwords → types, error

Level 3 (Index):
  - index → corpus, tokenizer, stopwords, types
  - scorer → types

Level 4 (Query):
  - query → index, scorer, types
  - graph → index, types

Level 5 (Interface):
  - cli → query, graph, corpus
  - api → query, graph (future)
  - viz → graph, types
```

**Rule**: Level N can only depend on Level < N (acyclic)

### 3.3 Trait Structure

```rust
// Corpus abstraction
trait CorpusSource {
    fn discover(&self) -> Result<Vec<PathBuf>>;
}

// Tokenization strategy
trait TokenizeStrategy {
    fn tokenize(&self, text: &[u8]) -> Vec<String>;
}

// Scoring strategy
trait ScoringFunction {
    fn score(&self, tf: f64, idf: f64) -> u128;
}

// Serialization
trait IndexStore {
    fn save(&self, index: &Index) -> Result<()>;
    fn load(&self) -> Result<Index>;
}

// Graph construction
trait GraphBuilder {
    fn build_graph(&self, index: &Index) -> Graph;
}
```

---

## 4. SERVICES VIEW

### 4.1 Current Interface (Phase 1)

**CLI Service**
```
tfidf [OPTIONS] <COMMAND>

Commands:
  similar     Find similar documents
  terms       Show top terms
  links       Generate backlinks
  save        Build index
  query       Query saved index
```

### 4.2 Future Interfaces

**Library API (Phase 1)**
```rust
pub mod corpus;     // File discovery
pub mod tokenizer;  // Text processing
pub mod index;      // TF-IDF index
pub mod query;      // Similarity queries
pub mod graph;      // Hypergraph construction
```

**HTTP API (Phase 3)**
```
POST /api/v1/index/build
POST /api/v1/query/similar
GET  /api/v1/query/terms
POST /api/v1/graph/generate
```

**WebSocket API (Phase 3)**
```
WS /ws/v1/stream
  → {"type": "similar", "file": "...", "corpus": "..."}
  ← {"type": "result", "docs": [...]}
```

---

## 5. DATA VIEW

### 5.1 Information Flow

```d2
direction: right

files: File System {
  shape: cylinder
}

tokenizer: Tokenizer {
  shape: rectangle
}

index: Index {
  shape: rectangle
  
  docs: Documents []
  term_df: Term→DF Map
  doc_vectors: Doc→Vector Map
}

query: Query Engine {
  shape: rectangle
}

graph: Graph {
  shape: rectangle
  
  nodes: Nodes (docs, terms)
  edges: Edges (similarity, co-occurrence)
}

results: Results {
  shape: document
}

files -> tokenizer: Read bytes
tokenizer -> index: Tokens
index -> query: TF-IDF Vectors
index -> graph: Term Relations
query -> results: Ranked List
graph -> results: D2 Diagram
```

### 5.2 Data Structures

**Document**
```rust
struct Document {
    id: DocId,           // UUID
    path: PathBuf,       // Original location
    corpus: String,      // notes, wiki, transcripts
    terms: HashMap<String, u32>,  // term → count
    length: u32,         // total tokens
    metadata: Metadata,  // timestamps, tags, etc.
}
```

**Index**
```rust
struct Index {
    docs: Vec<Document>,
    term_df: HashMap<String, u32>,         // document frequency
    term_idf: HashMap<String, f64>,        // pre-computed IDF
    doc_vectors: HashMap<DocId, Vector>,   // sparse TF-IDF vectors
    corpus_name: String,
    created_at: DateTime<Utc>,
}
```

**Graph**
```rust
struct Hypergraph {
    nodes: HashMap<NodeId, Node>,
    edges: Vec<Hyperedge>,
    metadata: GraphMetadata,
}

enum Node {
    Document(DocId),
    Term(String),
    Concept(String),  // from clustering
}

struct Hyperedge {
    nodes: Vec<NodeId>,
    weight: f64,
    edge_type: EdgeType,
}

enum EdgeType {
    Similarity,      // doc-doc
    Contains,        // doc-term
    CoOccurrence,    // term-term
    Subsumes,        // concept-term
}
```

### 5.3 Storage Formats

**Index Files** (`.tidx` extension)
```
Format: Bincode v1
Location: ~/.local/share/tfidf/indices/
Naming: {corpus_name}_{timestamp}.tidx
Compression: Optional (zstd)
```

**Cache Files**
```
Format: MessagePack
Location: ~/.cache/tfidf/
TTL: 24 hours
Eviction: LRU
```

**Config Files**
```toml
# ~/.config/tfidf/config.toml
[corpus.notes]
root = "~/notes"
extensions = ["md", "org"]
stopwords = "~/.config/tfidf/stopwords/english.txt"

[corpus.wiki]
root = "~/wiki"
extensions = ["md"]

[corpus.transcripts]
root = "~/transcripts"
extensions = ["txt", "md"]

[index]
cache_ttl = "24h"
auto_rebuild = false

[query]
default_top_n = 10
similarity_threshold = 0.1
```

---

## 6. STANDARDS VIEW

### 6.1 Technology Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Language | Rust 1.75+ | Memory safety, performance, zero-cost abstractions |
| Build | Cargo + build.rs | Standard Rust toolchain |
| CLI | clap 4.x | Derive macros, shell completions, man pages |
| Serialization | bincode + serde | Fast, compact, type-safe |
| Parallelism | rayon | Data parallelism, automatic |
| Testing | cargo test | Built-in |
| Benchmarking | criterion | Statistical rigor |
| Documentation | rustdoc | Integrated, markdown-based |
| Diagrams | d2 | Programmable, version-controllable |
| Packaging | build.rs → pkgbuild | Arch Linux packages from tags |

### 6.2 Code Standards

**Naming Conventions**
```rust
// Types: PascalCase
struct Document { }
enum NodeType { }

// Functions: snake_case
fn tokenize(text: &[u8]) -> Vec<String>

// Constants: SCREAMING_SNAKE_CASE
const MAX_TERM_LENGTH: usize = 100;

// Modules: snake_case
mod tokenizer;
mod query;
```

**Module Organization**
```
src/
  lib.rs              // Public API
  types/              // Core types (Level 0)
    document.rs
    term.rs
    score.rs
  error.rs            // Error types (Level 0)
  tokenizer/          // Tokenization (Level 1)
    mod.rs
    ascii.rs
    stopwords.rs
  corpus/             // File discovery (Level 2)
    mod.rs
    discover.rs
    filter.rs
  index/              // TF-IDF index (Level 3)
    mod.rs
    build.rs
    persist.rs
  query/              // Queries (Level 4)
    mod.rs
    similar.rs
    terms.rs
  graph/              // Hypergraph (Level 4)
    mod.rs
    build.rs
    export.rs
  cli/                // CLI interface (Level 5)
    mod.rs
    commands.rs
```

**Documentation Standards**
```rust
/// Brief one-line description.
///
/// Longer explanation of what this does, including:
/// - Key behaviors
/// - Edge cases
/// - Performance characteristics
///
/// # Examples
///
/// ```
/// use tfidf::tokenizer::tokenize;
///
/// let text = b"Hello, World!";
/// let tokens = tokenize(text);
/// assert_eq!(tokens, vec!["hello", "world"]);
/// ```
///
/// # Errors
///
/// Returns `Err` if...
///
/// # Panics
///
/// Panics if... (should be rare)
pub fn tokenize(text: &[u8]) -> Result<Vec<String>>
```

### 6.3 Testing Standards

**Unit Tests** (in same file)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_empty() {
        let result = tokenize(b"");
        assert_eq!(result, vec![]);
    }
}
```

**Integration Tests** (`tests/` directory)
```rust
// tests/similarity.rs
use tfidf::*;

#[test]
fn test_similar_documents() {
    let corpus = Corpus::discover("tests/fixtures").unwrap();
    let index = Index::build(&corpus).unwrap();
    let similar = index.similar_to("tests/fixtures/a.txt", 5).unwrap();
    assert!(similar.len() > 0);
}
```

**Benchmarks** (`benches/` directory)
```rust
// benches/tokenization.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_tokenize(c: &mut Criterion) {
    let text = include_bytes!("../tests/fixtures/large.txt");
    c.bench_function("tokenize_1mb", |b| {
        b.iter(|| tokenize(black_box(text)))
    });
}

criterion_group!(benches, bench_tokenize);
criterion_main!(benches);
```

---

## 7. PROJECT VIEW

### 7.1 Repository Structure

```
tfidf/
├── Cargo.toml              # Workspace root
├── Cargo.lock              # Locked dependencies
├── build.rs                # Build script (pkgbuild generation)
├── README.md               # User documentation
├── ARCHITECTURE.md         # This document
├── LICENSE                 # MIT or Apache-2.0
├── .gitignore
├── rustfmt.toml            # Code formatting
├── clippy.toml             # Linting rules
│
├── crates/
│   ├── tfidf/              # Core library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types/
│   │   │   ├── tokenizer/
│   │   │   ├── corpus/
│   │   │   ├── index/
│   │   │   ├── query/
│   │   │   └── graph/
│   │   ├── tests/
│   │   └── benches/
│   │
│   ├── tfidf-cli/          # CLI application
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── cli.rs
│   │   │   └── commands/
│   │   └── tests/
│   │
│   └── tfidf-viz/          # Visualization tools (Phase 2)
│       └── ...
│
├── scripts/
│   ├── rust-script/        # Standalone scripts
│   │   ├── tfidf-quick.rs  # Quick-win script
│   │   └── templates/      # Script templates
│   ├── install.sh          # Installation script
│   └── bench.sh            # Benchmark runner
│
├── docs/
│   ├── architecture/       # Architecture documentation
│   │   ├── capability.md
│   │   ├── operational.md
│   │   └── diagrams/       # D2 diagrams
│   ├── theory/             # Category theory docs
│   │   ├── resource-theory.md
│   │   ├── composition.md
│   │   └── hypergraphs.md
│   ├── guides/             # User guides
│   │   ├── quickstart.md
│   │   ├── configuration.md
│   │   └── examples/
│   └── api/                # API documentation
│
├── tests/
│   ├── fixtures/           # Test data
│   │   ├── corpus_a/
│   │   └── corpus_b/
│   └── integration/        # Integration tests
│
├── benches/
│   └── suite.rs            # Benchmark suite
│
├── pkg/
│   ├── arch/               # Arch Linux package
│   │   └── PKGBUILD.template
│   └── systemd/            # systemd service files (future)
│
└── target/                 # Build artifacts (gitignored)
```

### 7.2 Build Pipeline

**build.rs Responsibilities**
```rust
// build.rs
fn main() {
    // 1. Generate man pages from clap
    generate_man_pages();
    
    // 2. Generate shell completions
    generate_completions();
    
    // 3. Check for release tag
    if let Some(tag) = env::var("GIT_TAG").ok() {
        // 4. Generate PKGBUILD from template
        generate_pkgbuild(&tag);
    }
    
    // 5. Embed version info
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash());
}
```

**Release Process**
```bash
# 1. Tag release
git tag -a v0.1.0 -m "Quick win release"

# 2. Build with tag
GIT_TAG=v0.1.0 cargo build --release

# 3. Generate package
cd pkg/arch
makepkg -si

# 4. Test in systemd-nspawn
sudo systemd-nspawn -D ~/containers/archlinux \
  --bind $PWD/target/release/tfidf:/usr/local/bin/tfidf \
  /usr/local/bin/tfidf --help

# 5. Push to forgejo
git push origin v0.1.0
```

### 7.3 Development Workflow

**Quick Iteration Loop**
```bash
# 1. Use rust-script for rapid prototyping
./scripts/rust-script/tfidf-quick.rs similar test.md

# 2. Extract working code to library
# 3. Add tests
cargo test tokenizer::tests

# 4. Benchmark
cargo bench

# 5. Document
cargo doc --open

# 6. Lint
cargo clippy -- -D warnings

# 7. Format
cargo fmt

# 8. Commit
git commit -m "feat(tokenizer): add stopword filtering"
```

**Dogfooding Loop**
```bash
# Use tool to improve itself
tfidf similar src/tokenizer/mod.rs docs/ --ext md
# → Discover related documentation
# → Update docs based on discoveries
# → Commit improvements
# → Repeat
```

---

## 8. THEORETICAL VIEW

### 8.1 Category Theory Foundations

**Why Category Theory?**

Category theory provides:
1. **Composition laws**: Functions compose predictably
2. **Abstraction boundaries**: Clear interfaces via functors
3. **Type safety**: Morphisms preserve structure
4. **Reasoning tools**: Prove properties algebraically

**Core Concepts Applied**

**Objects**: Data types
- `Corpus` — collection of documents
- `Document` — bag of terms
- `Index` — TF-IDF representation
- `Graph` — hypergraph structure

**Morphisms**: Functions between objects
- `discover: Path → Corpus`
- `tokenize: Document → Vec<Term>`
- `build: Corpus → Index`
- `query: Index × Query → Results`

**Functors**: Structure-preserving maps
```rust
// Corpus is a functor over Vec<PathBuf>
impl Corpus {
    fn map<F>(&self, f: F) -> Corpus
    where F: Fn(&PathBuf) -> Option<PathBuf>
    {
        Corpus {
            files: self.files.iter().filter_map(f).collect()
        }
    }
}

// Identity law: corpus.map(id) == corpus
// Composition law: corpus.map(f).map(g) == corpus.map(g ∘ f)
```

**Natural Transformations**: Polymorphic functions
```rust
// Filter is a natural transformation
trait Filter {
    fn apply(&self, corpus: Corpus) -> Corpus;
}

// Extension filter
struct ExtFilter(Vec<String>);
impl Filter for ExtFilter {
    fn apply(&self, corpus: Corpus) -> Corpus {
        corpus.map(|p| {
            if has_extension(p, &self.0) { Some(p.clone()) }
            else { None }
        })
    }
}

// Composition of filters is a filter
impl<F: Filter, G: Filter> Filter for (F, G) {
    fn apply(&self, corpus: Corpus) -> Corpus {
        self.1.apply(self.0.apply(corpus))
    }
}
```

### 8.2 Resource Theory

**Resources**: Things that can be composed
- **Corpora**: Merge, intersect, diff
- **Indices**: Join, union
- **Graphs**: Overlay, product

**Operations**: How resources combine
```rust
// Monoidal structure on Corpus
impl Corpus {
    fn empty() -> Self { Corpus { files: vec![] } }
    
    fn union(self, other: Self) -> Self {
        let mut files = self.files;
        files.extend(other.files);
        Corpus { files }
    }
}

// Laws:
// - empty().union(c) == c
// - c.union(empty()) == c
// - a.union(b).union(c) == a.union(b.union(c))
```

**Capability as Resource**
```
Capability C requires Resources R = {r1, r2, ...}

Example:
  C1.1 (Similarity) requires:
    - Tokenizer
    - Index
    - Scorer
    - Corpus
  
  If we have Tokenizer + Corpus → we can build Index
  If we have Index → we can build Scorer
  ∴ We can achieve C1.1
```

### 8.3 Hypergraph Emergence

**Local Relations → Global Structure**

```
Term co-occurrence (local):
  "graph" co-occurs with "node" (weight: 0.8)
  "graph" co-occurs with "edge" (weight: 0.7)
  "node" co-occurs with "edge" (weight: 0.9)

Emergent cluster (global):
  {graph, node, edge} forms a topic cluster
  → Label: "Graph Theory"
```

**Hyperedge = Shared Context**
```
Documents: [d1, d2, d3]
Terms in d1: [graph, algorithm]
Terms in d2: [graph, visualization]
Terms in d3: [algorithm, complexity]

Hyperedge: {d1, d2, d3, graph, algorithm}
  → These docs share context about "graph algorithms"
```

**Topological Slices**
```rust
// Partition hypergraph by corpus
let notes_graph = hypergraph.slice(|node| {
    node.corpus() == "notes"
});

let wiki_graph = hypergraph.slice(|node| {
    node.corpus() == "wiki"
});

// Cross-corpus edges reveal connections
let bridges = hypergraph.edges_between(
    notes_graph.nodes(),
    wiki_graph.nodes()
);
// → These are the "backlinks" we want to generate
```

### 8.4 Non-AI Embeddings

**Traditional**: Use neural networks to learn embeddings

**Our Approach**: Derive embeddings from structure

```
1. Build term co-occurrence matrix M
   M[i,j] = PMI(term_i, term_j)  // Pointwise Mutual Information

2. Apply dimensionality reduction
   SVD(M) = U Σ V^T
   Embeddings = U[:, :k]  // First k dimensions

3. Document embeddings = weighted average of term embeddings
   doc_vec = Σ (tfidf(term) * term_embedding)
```

**Advantages**:
- Deterministic (no training)
- Interpretable (dimensions = latent topics)
- Fast (matrix operations)
- No external dependencies

**Alignment with User**:
- Embeddings reflect YOUR corpus
- No generic pre-trained model
- Evolves as you write more

---

## 9. STOPWORDS EXPLANATION

### Why Stopwords Exist

**Problem**: High-frequency, low-information words

Example corpus:
```
doc1: "the cat sat on the mat"
doc2: "the dog sat on the log"
doc3: "the bird flew over the tree"
```

Without stopwords, top terms:
```
1. "the" (appears 6 times)
2. "sat", "on" (appears 2 times each)
3. "cat", "dog", "mat", "log", "bird", "flew", "over", "tree" (1 each)
```

With stopwords (`["the", "on", "over"]`):
```
1. "sat" (appears 2 times)
2. "cat", "dog", "mat", "log", "bird", "flew", "tree" (1 each)
```

**Result**: More meaningful terms rise to the top

### When to Use Stopwords

**Use stopwords when**:
- Analyzing natural language text (notes, transcripts, articles)
- You want to find content-bearing terms
- Corpus contains lots of English prose

**Don't use stopwords when**:
- Analyzing code (keywords like "for", "if" are meaningful)
- Technical documentation (function words provide context)
- Short texts (tweets, titles)

### Stopword Strategy

**Option 1**: Standard English list
```
# ~/.config/tfidf/stopwords/english.txt
the
a
an
is
are
was
were
...
```

**Option 2**: Corpus-specific (learned)
```bash
# Find most common terms across corpus
tfidf terms corpus/ --top 100

# Manually select non-informative ones
# Add to stopwords file
```

**Option 3**: Domain-specific
```
# For code+docs corpus, keep technical terms
# Remove only articles, pronouns, conjunctions
```

### Implementation

```rust
// Stopwords are just a HashSet lookup
fn tokenize(text: &[u8], stopwords: &HashSet<String>) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in extract_words(text) {
        if !stopwords.contains(&word) {
            tokens.push(word);
        }
    }
    tokens
}
```

**Performance**: O(1) lookup per term (hash table)

---

## 10. DIAGRAMS

### 10.1 System Context

```d2
direction: right

user: Developer {
  shape: person
}

system: TF-IDF System {
  style.border-radius: 10
}

notes: Notes Corpus {
  shape: cylinder
}

wiki: Wiki Corpus {
  shape: cylinder
}

transcripts: Transcripts {
  shape: cylinder
}

viz: Visualization Tools {
  shape: rectangle
}

user -> system: Queries
system -> notes: Index & Query
system -> wiki: Index & Query
system -> transcripts: Index & Query
system -> viz: Generate Diagrams
viz -> user: D2, Graphviz, etc.
```

### 10.2 Component Dependencies (Layered)

> **Issue**: Cannot create layers between board.
> 
> *How to resolve*
> - Review the necessity and purpose of diagram.
> - Specify its utility or remove it.

```d2
layers: {
  l0: Level 0 - Types {
    types: types
    error: error
  }
  
  l1: Level 1 - Foundation {
    tokenizer: tokenizer
    hasher: hasher
    glob: glob
  }
  
  l2: Level 2 - Core {
    corpus: corpus
    stopwords: stopwords
  }
  
  l3: Level 3 - Index {
    index: index
    scorer: scorer
  }
  
  l4: Level 4 - Query {
    query: query
    graph: graph
  }
  
  l5: Level 5 - Interface {
    cli: cli
    api: api
  }
}

layers.l1.tokenizer -> layers.l0.types
layers.l1.tokenizer -> layers.l0.error
layers.l2.corpus -> layers.l1.tokenizer
layers.l2.corpus -> layers.l1.glob
layers.l3.index -> layers.l2.corpus
layers.l3.index -> layers.l2.stopwords
layers.l4.query -> layers.l3.index
layers.l4.graph -> layers.l3.index
layers.l5.cli -> layers.l4.query
layers.l5.cli -> layers.l4.graph
```

### 10.3 Data Flow (Indexing)

```d2
direction: right

files: Files {
  shape: cylinder
}

discover: Discover Files {
  shape: diamond
}

corpus: Corpus

filter: Apply Filters {
  shape: diamond
}

tokenize: Tokenize {
  shape: rectangle
}

docs: Documents

build: Build Index {
  shape: rectangle
}

index: Index {
  shape: cylinder
}

files -> discover
discover -> corpus
corpus -> filter
filter -> tokenize
tokenize -> docs
docs -> build
build -> index
```

### 10.4 Query Flow (Similarity)

```d2
direction: right

query: User Query {
  shape: document
}

load: Load Index {
  shape: rectangle
}

index: Index {
  shape: cylinder
}

target: Find Target {
  shape: diamond
}

compute: Compute Vectors {
  shape: rectangle
}

cosine: Cosine Similarity {
  shape: rectangle
}

rank: Rank Results {
  shape: rectangle
}

results: Results {
  shape: document
}

query -> load
load -> index
index -> target
target -> compute
compute -> cosine
cosine -> rank
rank -> results
```

---

## 11. CAPABILITY ROADMAP (Detailed)

### Phase 1: Quick Win (2 weeks)

**Goal**: Working tool for similarity queries

**Deliverables**:
- [x] Rust-script (immediate use)
- [ ] Core library crate
- [ ] CLI with clap
- [ ] Basic tests (>80% coverage)
- [ ] Documentation (README, rustdoc)
- [ ] Benchmark baseline

**Success Criteria**:
- Can find similar documents in <2s
- Can save/load index
- Man page generated
- Shell completions work

### Phase 2: Hypergraph (4 weeks)

**Goal**: Multi-corpus linking and graph construction

**Deliverables**:
- [ ] Cross-corpus query support
- [ ] Term co-occurrence matrix
- [ ] Hypergraph construction
- [ ] D2 diagram generation
- [ ] Backlink generation
- [ ] Topic modeling (basic clustering)

**Success Criteria**:
- Can link notes ↔ wiki
- Can generate concept graphs
- D2 diagrams render correctly
- Emergent clusters make sense

### Phase 3: Ecosystem (6 weeks)

**Goal**: Multi-interface support

**Deliverables**:
- [ ] HTTP API server
- [ ] WebSocket support
- [ ] Web interface (terminal in browser?)
- [ ] Neovim plugin
- [ ] Advanced clustering
- [ ] Interactive visualization

**Success Criteria**:
- Feature parity across terminal/web/neovim
- <100ms API response time
- Real-time updates via WebSocket

---

## 12. LEARNINGS CAPTURE

### 12.1 Decision Log

| Date | Decision | Rationale | Outcome |
|------|----------|-----------|---------|
| 2025-12-01 | Multi-file over single-file | Manage complexity | TBD |
| 2025-12-01 | Keep FastHasher | 5-10% speedup | TBD |
| 2025-12-01 | clap for CLI | Man pages, completions | TBD |

### 12.2 Performance Metrics

Track these over time:

```
# Indexing performance
- Files indexed per second
- Memory usage during indexing
- Time to save/load index

# Query performance
- Similarity query latency (p50, p95, p99)
- Terms query latency
- Memory usage during query

# Code health
- Lines of code (total, by module)
- Test coverage percentage
- Cyclomatic complexity
- Dependency count
```

### 12.3 Feedback Integration

```bash
# Use tool to find related issues
tfidf similar issues/bug-123.md issues/ --ext md

# Use tool to find relevant docs
tfidf similar src/tokenizer/mod.rs docs/ --ext md

# Use tool to discover missing documentation
tfidf terms src/ --ext rs --top 50 > /tmp/code-terms.txt
tfidf terms docs/ --ext md --top 50 > /tmp/doc-terms.txt
comm -23 /tmp/code-terms.txt /tmp/doc-terms.txt
# → Terms in code but not docs = need documentation
```

---

## 13. NEXT ACTIONS

### Immediate (Today)

1. **Review this document**: Approve/modify architecture
2. **Use rust-script**: Start getting value immediately
3. **Initialize repo**: Create project structure
4. **First commit**: Add architecture docs + rust-script

### This Week

5. **Scaffold project**: Create crate structure
6. **Extract modules**: Move rust-script code to library
7. **Add tests**: Unit tests for core functions
8. **Add CLI**: clap-based interface

### This Month

9. **Phase 1 complete**: Working tool with docs
10. **Start dogfooding**: Use tool for development
11. **Measure baseline**: Benchmark performance
12. **Plan Phase 2**: Based on usage patterns

---

## APPENDIX A: Glossary

- **Corpus**: Collection of documents to analyze
- **TF-IDF**: Term Frequency-Inverse Document Frequency
- **Hypergraph**: Graph where edges can connect >2 nodes
- **Topological Slice**: Subset of graph by property
- **Functor**: Structure-preserving map between categories
- **Natural Transformation**: Map between functors
- **Monoidal Category**: Category with tensor product ⊗
- **PMI**: Pointwise Mutual Information
- **SVD**: Singular Value Decomposition

## APPENDIX B: References

- [Category Theory for Programmers](https://bartoszmilewski.com/2014/10/28/category-theory-for-programmers-the-preface/)
- [Resource Theories](https://arxiv.org/abs/1409.5531)
- [Hypergraph Theory](https://en.wikipedia.org/wiki/Hypergraph)
- [Information Retrieval](https://nlp.stanford.edu/IR-book/)

---

**END OF ARCHITECTURE DOCUMENT**

*This is a living document. Update as system evolves.*
