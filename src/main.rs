// A composable TF-IDF index for pipe-friendly text analysis.
//
// Usage:
//   tfidf save [--index <path>] [PATH...] [--]
//   tfidf query [--index <path>] {terms|similar|list|search} [args...]
//
// Requirements:
//   - Tab-delimited output suitable for cut, sort, xargs
//   - Graceful handling of pipe termination (head, tail)
//   - Index path resolution: --index > TFIDF_INDEX > data dir
//   - XDG paths use TFIDF_APPNAME prefix for multi-instance support
//   - Stdin path mixing after -- for composition
//   - No decoration, progress output, or hidden state writes in normal operation
//
// Design:
//   Functions compose through clear interfaces. The Index forms a monoid
//   over document collections (though this implementation doesn't yet expose
//   the associative structure for incremental updates). Similarity forms a
//   metric space over the vector representation.
//
//   Tokenization is a functor from byte sequences to term multisets. The
//   lowercase normalization is an equivalence relation over terms.
//
//   TF-IDF vectors form a linear map from the term space to ℝⁿ. Cosine
//   similarity is the canonical inner product on the unit sphere, making
//   it a proper metric for comparing direction regardless of magnitude.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::hash::{BuildHasherDefault, Hasher};
use std::io::{self, BufRead, BufReader, BufWriter, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const DEFAULT_APP_NAME: &str = "tfidf";
const DEFAULT_INDEX_FILE: &str = "tfidf.index";

// ============================================================================
// Fast Hasher
// ============================================================================

/// Custom hash function optimized for short strings (terms).
/// Uses a mixing function inspired by splitmix64.
#[derive(Default)]
struct FastHasher(u64);

impl Hasher for FastHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut s = self.0 ^ 0x9E37_79B9_7F4A_7C15;
        for chunk in bytes.chunks(8) {
            let v = match chunk.len() {
                8 => u64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]),
                _ => {
                    let mut v = 0u64;
                    for (i, &b) in chunk.iter().enumerate() {
                        v |= (b as u64) << (i * 8);
                    }
                    v
                }
            };
            let mut h = v;
            h ^= h >> 30;
            h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 27;
            h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
            s ^= h;
            s = s.rotate_left(23).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
        self.0 = s;
    }

    fn finish(&self) -> u64 {
        let mut z = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z ^= z >> 31;
        z = z.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z ^= z >> 27;
        z = z.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        z ^= z >> 33;
        z
    }
}

type FastBuild = BuildHasherDefault<FastHasher>;
type FastMap<K, V> = HashMap<K, V, FastBuild>;

// ============================================================================
// Output handling
// ============================================================================

/// Write output with graceful handling of broken pipes.
/// When piped to head/tail/etc, the pipe may close before we finish writing.
/// This is expected behavior and should exit cleanly without error messages.
struct PipeWriter {
    writer: BufWriter<io::Stdout>,
}

impl PipeWriter {
    fn new() -> Self {
        PipeWriter {
            writer: BufWriter::new(io::stdout()),
        }
    }

    fn writeln(&mut self, s: &str) -> Result<()> {
        match writeln!(self.writer, "{}", s) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                std::process::exit(0);
            }
            Err(e) => Err(e.into()),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self.writer.flush() {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                std::process::exit(0);
            }
            Err(e) => Err(e.into()),
        }
    }
}

// ============================================================================
// Tokenization
// ============================================================================

/// Predicate: byte is part of a word token (ASCII alphanumeric or underscore).
#[inline]
fn is_word_byte(b: u8) -> bool {
    matches!(b | 0x20, b'a'..=b'z') || b.is_ascii_digit() || b == b'_'
}

/// Extract lowercase word tokens from bytes, filtering stopwords.
///
/// Tokenization forms a functor from byte sequences to term multisets.
/// The lowercase normalization is an equivalence relation over terms.
fn tokenize(text: &[u8], stopwords: &FastMap<String, ()>) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = Vec::new();

    for &b in text {
        if is_word_byte(b) {
            current.push(b | 0x20); // ASCII lowercase
        } else if !current.is_empty() {
            if let Ok(word) = String::from_utf8(std::mem::take(&mut current)) {
                if !stopwords.contains_key(&word) && !word.is_empty() {
                    tokens.push(word);
                }
            }
        }
    }

    if !current.is_empty() {
        if let Ok(word) = String::from_utf8(current) {
            if !stopwords.contains_key(&word) && !word.is_empty() {
                tokens.push(word);
            }
        }
    }

    tokens
}

/// Resolve application name for XDG paths.
/// Uses TFIDF_APPNAME if set, otherwise defaults to "tfidf".
/// This allows multiple independent instances with different configurations.
fn app_name() -> String {
    env::var("TFIDF_APPNAME")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_APP_NAME.to_string())
}

/// Load stopwords from standard config locations.
/// Returns empty set if no stopwords file exists.
///
/// Search order:
///   1. $XDG_CONFIG_HOME/${TFIDF_APPNAME}/stopwords.txt
///   2. ~/.config/${TFIDF_APPNAME}/stopwords.txt
fn load_stopwords() -> Result<FastMap<String, ()>> {
    let app = app_name();

    let paths = [
        env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|x| PathBuf::from(x).join(&app).join("stopwords.txt")),
        env::var("HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|h| {
                PathBuf::from(h)
                    .join(".config")
                    .join(&app)
                    .join("stopwords.txt")
            }),
    ];

    for path in paths.into_iter().flatten() {
        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            let mut set = FastMap::default();
            for line in reader.lines() {
                let word = line?.trim().to_lowercase();
                if !word.is_empty() {
                    set.insert(word, ());
                }
            }
            return Ok(set);
        }
    }

    Ok(FastMap::default())
}

// ============================================================================
// XDG paths
// ============================================================================

fn cwd_or_dot() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Resolve the TF-IDF data directory.
///
/// Precedence:
///   1. $TFIDF_DATA_DIR
///   2. $XDG_DATA_HOME/$TFIDF_APPNAME
///   3. $HOME/.local/share/$TFIDF_APPNAME
///   4. ./$TFIDF_APPNAME
///
/// $TFIDF_APPNAME defaults to "tfidf".
fn tfidf_data_dir() -> PathBuf {
    if let Some(path) = env_path("TFIDF_DATA_DIR") {
        return path;
    }

    let app = app_name();

    if let Some(path) = env_path("XDG_DATA_HOME") {
        return path.join(app);
    }

    home_dir()
        .map(|home| home.join(".local").join("share").join(&app))
        .unwrap_or_else(|| cwd_or_dot().join(app))
}

/// Get default data path for index.
fn xdg_index_path() -> PathBuf {
    tfidf_data_dir().join(DEFAULT_INDEX_FILE)
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

// ============================================================================
// Corpus Discovery
// ============================================================================

/// A collection of file paths forming a corpus.
/// This is a simple wrapper over Vec<PathBuf> that could be extended
/// to support filtering, sampling, or other collection operations.
struct Corpus {
    files: Vec<PathBuf>,
}

impl Corpus {
    /// Build corpus from paths, recursively walking directories.
    /// Forms a natural transformation from filesystem paths to document collections.
    fn from_paths<I>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut files = Vec::new();
        for p in paths {
            if p.is_dir() {
                Self::walk_dir(&p, &mut files)?;
            } else if p.is_file() {
                files.push(p);
            }
        }
        Ok(Corpus { files })
    }

    fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                Self::walk_dir(&path, files)?;
            } else if path.is_file() {
                files.push(path);
            }
        }
        Ok(())
    }
}

// ============================================================================
// Index
// ============================================================================

type TermFreq = FastMap<String, u32>;

/// A document with its term frequency distribution.
/// The terms form a multiset, represented as a frequency map.
#[derive(Clone, Serialize, Deserialize)]
struct Document {
    id: u32,
    path: String,
    terms: TermFreq,
    length: u32,
}

/// The index maintains the invariant that term_df[t] equals the number
/// of documents containing term t. This allows efficient IDF computation.
///
/// Future extension: This structure naturally extends to a monoid where
/// the binary operation is index union (with DF recomputation). The identity
/// is the empty index.
#[derive(Serialize, Deserialize)]
struct Index {
    docs: Vec<Document>,
    term_df: FastMap<String, u32>,
}

impl Index {
    /// Build index from corpus. This is a pure function: same corpus yields same index.
    fn build(corpus: &Corpus, stopwords: &FastMap<String, ()>) -> Result<Self> {
        let docs: Vec<Document> = corpus
            .files
            .par_iter()
            .enumerate()
            .filter_map(|(i, path)| {
                let bytes = fs::read(path).ok()?;
                let tokens = tokenize(&bytes, stopwords);

                if tokens.is_empty() {
                    return None;
                }

                // Build term frequency map (multiset representation)
                let mut terms: TermFreq = FastMap::default();
                for token in tokens {
                    *terms.entry(token).or_insert(0) += 1;
                }

                let length = terms.values().sum();

                Some(Document {
                    id: i as u32,
                    path: path.to_string_lossy().into_owned(),
                    terms,
                    length,
                })
            })
            .collect();

        // Compute document frequencies: for each term, count containing documents
        let mut term_df = FastMap::default();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        Ok(Index { docs, term_df })
    }

    /// Serialize index to disk.
    fn save(&self, path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }

    /// Deserialize index from disk.
    fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(bincode::deserialize_from(reader)?)
    }

    /// Compute TF-IDF score for a term in a document.
    ///
    /// TF-IDF = TF(t,d) × IDF(t)
    /// where:
    ///   TF(t,d) = count(t,d) / |d|
    ///   IDF(t) = ln((D + 1) / (df(t) + 1))
    ///
    /// The smoothing (+1) prevents division by zero and handles unseen terms.
    fn tfidf(&self, term: &str, count: u32, doc_len: u32) -> f64 {
        let df = self.term_df.get(term).copied().unwrap_or(1) as f64;
        let num_docs = self.docs.len().max(1) as f64;

        let idf = ((num_docs + 1.0) / (df + 1.0)).ln().max(0.0);
        let tf = count as f64 / doc_len.max(1) as f64;

        tf * idf
    }

    /// Build TF-IDF vector representation for a document.
    /// This is a linear map from the term space to ℝⁿ.
    fn vector(&self, doc: &Document) -> FastMap<String, f64> {
        doc.terms
            .iter()
            .map(|(term, &count)| (term.clone(), self.tfidf(term, count, doc.length)))
            .collect()
    }

    /// Iterate over document paths in the index.
    fn list_paths(&self) -> impl Iterator<Item = &str> {
        self.docs.iter().map(|d| d.path.as_str())
    }

    /// Extract top N terms by aggregate TF-IDF score across all documents.
    fn top_terms(&self, n: usize) -> Vec<(String, f64)> {
        let mut scores: FastMap<String, f64> = FastMap::default();

        for doc in &self.docs {
            for (term, &count) in &doc.terms {
                let score = self.tfidf(term, count, doc.length);
                *scores.entry(term.clone()).or_insert(0.0) += score;
            }
        }

        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(n);
        ranked
    }

    /// Find documents similar to target using cosine similarity.
    ///
    /// Cosine similarity forms a metric on the vector space of TF-IDF representations.
    /// It's invariant to document length scaling, making it suitable for comparing
    /// documents of different sizes.
    fn similar_to(&self, target: &str, n: usize) -> Result<Vec<(String, f64)>> {
        let target_doc = self
            .docs
            .iter()
            .find(|d| d.path == target)
            .ok_or_else(|| format!("target not found: {}", target))?;

        let target_vec = self.vector(target_doc);

        let mut scores: Vec<_> = self
            .docs
            .par_iter()
            .filter(|d| d.id != target_doc.id)
            .map(|doc| {
                let vec = self.vector(doc);
                let sim = cosine_similarity(&target_vec, &vec);
                (doc.path.clone(), sim)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);

        Ok(scores)
    }

    /// Search documents by query string.
    /// Tokenizes the query and computes similarity against all documents.
    fn search(&self, query: &str, n: usize, stopwords: &FastMap<String, ()>) -> Vec<(String, f64)> {
        let query_tokens = tokenize(query.as_bytes(), stopwords);

        if query_tokens.is_empty() {
            return Vec::new();
        }

        let mut query_terms: TermFreq = FastMap::default();
        for token in query_tokens {
            *query_terms.entry(token).or_insert(0) += 1;
        }

        let query_len = query_terms.values().sum();
        let query_vec: FastMap<String, f64> = query_terms
            .iter()
            .map(|(term, &count)| (term.clone(), self.tfidf(term, count, query_len)))
            .collect();

        let mut scores: Vec<_> = self
            .docs
            .par_iter()
            .map(|doc| {
                let vec = self.vector(doc);
                let sim = cosine_similarity(&query_vec, &vec);
                (doc.path.clone(), sim)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);

        scores
    }
}

/// Compute cosine similarity between two sparse vectors.
///
/// cos(a,b) = (a·b) / (||a|| ||b||)
///
/// This is the canonical inner product on the unit sphere, making it
/// a proper metric for comparing direction regardless of magnitude.
fn cosine_similarity(a: &FastMap<String, f64>, b: &FastMap<String, f64>) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (term, &score_a) in a {
        norm_a += score_a * score_a;
        if let Some(&score_b) = b.get(term) {
            dot += score_a * score_b;
        }
    }

    for &score_b in b.values() {
        norm_b += score_b * score_b;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

// ============================================================================
// CLI
// ============================================================================

fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help") {
        print_usage();
        return Ok(());
    }

    match args.remove(0).as_str() {
        "save" => cmd_save(args),
        "query" => cmd_query(args),
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}

fn take_index_flag(args: &mut Vec<String>) -> Option<String> {
    let mut index_arg = None;
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--index" {
            if i + 1 >= args.len() {
                eprintln!("--index requires a path");
                std::process::exit(1);
            }
            index_arg = Some(args.remove(i + 1));
            args.remove(i);
        } else {
            i += 1;
        }
    }

    index_arg
}

fn should_treat_first_save_arg_as_legacy_index(first: &str, arg_count: usize) -> bool {
    if arg_count < 2 {
        return false;
    }

    let path = Path::new(first);

    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "index" || name == DEFAULT_INDEX_FILE)
    {
        return true;
    }

    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "index")
    {
        return true;
    }

    !path.exists()
}

/// Build and save an index from the given paths.
/// Paths after -- are read from stdin, one per line.
///
/// Supported forms:
///   tfidf save [--index <path>] [PATH...] [--]
///   tfidf save <index> [PATH...] [--]      # legacy positional index
fn cmd_save(mut args: Vec<String>) -> Result<()> {
    let explicit_index = take_index_flag(&mut args);

    let mut paths = Vec::new();
    let mut positional = Vec::new();
    let mut read_stdin = false;

    for arg in args {
        if arg == "--" {
            read_stdin = true;
            break;
        }
        positional.push(arg);
    }

    let index_path = if let Some(index) = explicit_index {
        PathBuf::from(index)
    } else if positional
        .first()
        .is_some_and(|first| should_treat_first_save_arg_as_legacy_index(first, positional.len()))
    {
        PathBuf::from(positional.remove(0))
    } else {
        resolve_index_path(None)
    };

    for arg in positional {
        paths.push(PathBuf::from(arg));
    }

    if read_stdin {
        for line in io::stdin().lock().lines() {
            let line = line?.trim().to_string();
            if !line.is_empty() {
                paths.push(PathBuf::from(line));
            }
        }
    }

    if paths.is_empty() {
        eprintln!("tfidf save: no input paths");
        eprintln!("usage: tfidf save [--index <path>] [PATH...] [--]");
        std::process::exit(1);
    }

    let stopwords = load_stopwords()?;
    let corpus = Corpus::from_paths(paths)?;
    let index = Index::build(&corpus, &stopwords)?;

    index.save(&index_path)?;

    Ok(())
}

/// Execute a query against a saved index.
fn cmd_query(mut args: Vec<String>) -> Result<()> {
    let index_arg = take_index_flag(&mut args);
    let index_path = resolve_index_path(index_arg.as_deref());

    if !index_path.exists() {
        eprintln!("index not found: {}", index_path.display());
        std::process::exit(1);
    }

    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let index = Index::load(&index_path)?;
    let subcmd = args.remove(0);

    match subcmd.as_str() {
        "list" => cmd_list(index),
        "terms" => cmd_terms(index, args),
        "similar" => cmd_similar(index, args),
        "search" => cmd_search(index, args),
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}

/// Resolve index path from explicit flag, environment, or the XDG data default.
///
/// Precedence:
///   1. explicit path
///   2. TFIDF_INDEX
///   3. data-dir default path
fn resolve_index_path(explicit: Option<&str>) -> PathBuf {
    if let Some(path) = explicit {
        return PathBuf::from(path);
    }

    if let Some(path) = env_path("TFIDF_INDEX") {
        return path;
    }

    xdg_index_path()
}

/// Parse --top N flag, returning default if not present.
fn parse_top(args: &[String], default: usize) -> usize {
    args.windows(2)
        .find(|w| w[0] == "--top")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(default)
}

fn cmd_list(index: Index) -> Result<()> {
    let mut out = PipeWriter::new();
    for path in index.list_paths() {
        out.writeln(path)?;
    }
    out.flush()?;
    Ok(())
}

fn cmd_terms(index: Index, args: Vec<String>) -> Result<()> {
    let n = parse_top(&args, 20);
    let terms = index.top_terms(n);
    let mut out = PipeWriter::new();

    for (term, score) in terms {
        out.writeln(&format!("{}\t{:.6}", term, score))?;
    }
    out.flush()?;

    Ok(())
}

fn cmd_similar(index: Index, args: Vec<String>) -> Result<()> {
    if args.is_empty() {
        eprintln!("tfidf query similar <file> [--top N]");
        std::process::exit(1);
    }

    let target = &args[0];
    let n = parse_top(&args[1..], 10);
    let similar = index.similar_to(target, n)?;
    let mut out = PipeWriter::new();

    for (path, score) in similar {
        out.writeln(&format!("{}\t{:.6}", path, score))?;
    }
    out.flush()?;

    Ok(())
}

fn cmd_search(index: Index, args: Vec<String>) -> Result<()> {
    let n = parse_top(&args, 10);
    let stopwords = load_stopwords()?;

    // Read query from stdin if args are empty, otherwise use first arg
    let query = if args.is_empty() || args[0] == "--top" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else {
        args[0].clone()
    };

    if query.is_empty() {
        eprintln!("tfidf query search: empty query");
        std::process::exit(1);
    }

    let results = index.search(&query, n, &stopwords);
    let mut out = PipeWriter::new();

    for (path, score) in results {
        out.writeln(&format!("{}\t{:.6}", path, score))?;
    }
    out.flush()?;

    Ok(())
}

fn print_usage() {
    eprintln!(
        "\
tfidf - composable document similarity

USAGE
  tfidf save [--index <path>] [PATH...] [--]
  tfidf query [--index <path>] <subcommand>

SAVE
  Build an index from files and directories.
  If --index is not provided, the index path is resolved using the normal
  index resolution cascade and will be created if it does not exist.
  After --, read additional paths from stdin (one per line).

QUERY SUBCOMMANDS
  list                      List all indexed documents
  terms [--top N]           Show top N terms (default: 20)
  similar <file> [--top N]  Find similar documents (default: 10)
  search [query] [--top N]  Search by query string (default: 10)
                            Reads from stdin if query not provided

INDEX RESOLUTION
  1. --index flag
  2. TFIDF_INDEX environment variable
  3. $TFIDF_DATA_DIR/tfidf.index

TFIDF_DATA_DIR RESOLUTION
  1. TFIDF_DATA_DIR
  2. XDG_DATA_HOME/TFIDF_APPNAME
  3. HOME/.local/share/TFIDF_APPNAME

CONFIGURATION
  Stopwords: $XDG_CONFIG_HOME/$TFIDF_APPNAME/stopwords.txt
             (defaults to $XDG_CONFIG_HOME/tfidf/stopwords.txt)

  Set TFIDF_APPNAME to use separate configurations:
    export TFIDF_APPNAME=project-docs
    mkdir -p ~/.config/project-docs

OUTPUT
  Tab-delimited for composition with cut, sort, xargs, etc.

EXAMPLES
  # Save index using the default resolved index path
  tfidf save src/ notes/ --

  # Save index explicitly
  tfidf save --index docs.index src/ notes/ --
  tfidf save docs.index src/ notes/ --    # legacy positional index form

  # Query
  tfidf query list | head -5
  tfidf query terms --top 100 | sort -k2,2nr
  tfidf query similar main.rs | cut -f1 | xargs cat
  tfidf query search \"error handling rust\" --top 20
  echo \"memory management\" | tfidf query search --top 10

  # Use separate configuration
  TFIDF_APPNAME=work tfidf save --index work.index ~/work/docs
"
    );
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_word_byte() {
        assert!(is_word_byte(b'a'));
        assert!(is_word_byte(b'Z'));
        assert!(is_word_byte(b'5'));
        assert!(is_word_byte(b'_'));
        assert!(!is_word_byte(b' '));
        assert!(!is_word_byte(b'.'));
        assert!(!is_word_byte(b'-'));
    }

    #[test]
    fn test_tokenize_basic() {
        let text = b"Hello World";
        let stopwords = FastMap::default();
        let tokens = tokenize(text, &stopwords);
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_with_stopwords() {
        let text = b"the quick brown fox";
        let mut stopwords = FastMap::default();
        stopwords.insert("the".to_string(), ());
        let tokens = tokenize(text, &stopwords);
        assert_eq!(tokens, vec!["quick", "brown", "fox"]);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let text = b"hello, world! test-case";
        let stopwords = FastMap::default();
        let tokens = tokenize(text, &stopwords);
        assert_eq!(tokens, vec!["hello", "world", "test", "case"]);
    }

    #[test]
    fn test_tokenize_numbers_and_underscores() {
        let text = b"test_case_123 and var_2";
        let stopwords = FastMap::default();
        let tokens = tokenize(text, &stopwords);
        assert_eq!(tokens, vec!["test_case_123", "and", "var_2"]);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut vec = FastMap::default();
        vec.insert("term1".to_string(), 1.0);
        vec.insert("term2".to_string(), 2.0);

        let sim = cosine_similarity(&vec, &vec);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut vec_a = FastMap::default();
        vec_a.insert("term1".to_string(), 1.0);

        let mut vec_b = FastMap::default();
        vec_b.insert("term2".to_string(), 1.0);

        let sim = cosine_similarity(&vec_a, &vec_b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_partial_overlap() {
        let mut vec_a = FastMap::default();
        vec_a.insert("common".to_string(), 1.0);
        vec_a.insert("unique_a".to_string(), 1.0);

        let mut vec_b = FastMap::default();
        vec_b.insert("common".to_string(), 1.0);
        vec_b.insert("unique_b".to_string(), 1.0);

        let sim = cosine_similarity(&vec_a, &vec_b);
        // cos(θ) where both vectors have norm √2 and dot product 1
        let expected = 1.0 / 2.0;
        assert!((sim - expected).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let vec_a = FastMap::default();
        let mut vec_b = FastMap::default();
        vec_b.insert("term".to_string(), 1.0);

        let sim = cosine_similarity(&vec_a, &vec_b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_parse_top_default() {
        let args = vec![];
        assert_eq!(parse_top(&args, 42), 42);
    }

    #[test]
    fn test_parse_top_explicit() {
        let args = vec!["--top".to_string(), "100".to_string()];
        assert_eq!(parse_top(&args, 42), 100);
    }

    #[test]
    fn test_parse_top_invalid() {
        let args = vec!["--top".to_string(), "invalid".to_string()];
        assert_eq!(parse_top(&args, 42), 42);
    }

    #[test]
    fn test_index_tfidf_computation() {
        // Create a minimal index for testing TF-IDF
        let mut doc1_terms = FastMap::default();
        doc1_terms.insert("rare".to_string(), 1);
        doc1_terms.insert("common".to_string(), 2);

        let mut doc2_terms = FastMap::default();
        doc2_terms.insert("common".to_string(), 1);

        let doc1 = Document {
            id: 0,
            path: "doc1".to_string(),
            terms: doc1_terms.clone(),
            length: 3,
        };

        let doc2 = Document {
            id: 1,
            path: "doc2".to_string(),
            terms: doc2_terms.clone(),
            length: 1,
        };

        let mut term_df = FastMap::default();
        term_df.insert("rare".to_string(), 1);
        term_df.insert("common".to_string(), 2);

        let index = Index {
            docs: vec![doc1, doc2],
            term_df,
        };

        // "rare" appears in 1 doc out of 2, "common" in 2 out of 2
        let rare_score = index.tfidf("rare", 1, 3);
        let common_score = index.tfidf("common", 2, 3);

        // rare should have higher IDF
        assert!(rare_score > common_score);
    }

    // ========================================================================
    // Benchmarks
    // ========================================================================
    // Run with: cargo test --release -- --nocapture bench
    // Or: cargo +nightly bench (if using #[bench] with test::Bencher)

    #[test]
    fn bench_tokenize_small() {
        let text = b"The quick brown fox jumps over the lazy dog";
        let stopwords = FastMap::default();
        let start = std::time::Instant::now();
        for _ in 0..10_000 {
            let _ = tokenize(text, &stopwords);
        }
        let elapsed = start.elapsed();
        println!("bench_tokenize_small: {:?} (10k iterations)", elapsed);
    }

    #[test]
    fn bench_tokenize_medium() {
        let text = b"Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";
        let mut stopwords = FastMap::default();
        stopwords.insert("the".to_string(), ());
        stopwords.insert("a".to_string(), ());
        stopwords.insert("an".to_string(), ());

        let start = std::time::Instant::now();
        for _ in 0..10_000 {
            let _ = tokenize(text, &stopwords);
        }
        let elapsed = start.elapsed();
        println!("bench_tokenize_medium: {:?} (10k iterations)", elapsed);
    }

    #[test]
    fn bench_cosine_similarity_sparse() {
        let mut vec_a = FastMap::default();
        let mut vec_b = FastMap::default();

        for i in 0..100 {
            vec_a.insert(format!("term_{}", i), i as f64);
            if i % 3 == 0 {
                vec_b.insert(format!("term_{}", i), (i * 2) as f64);
            }
        }

        let start = std::time::Instant::now();
        for _ in 0..100_000 {
            let _ = cosine_similarity(&vec_a, &vec_b);
        }
        let elapsed = start.elapsed();
        println!(
            "bench_cosine_similarity_sparse: {:?} (100k iterations)",
            elapsed
        );
    }

    #[test]
    fn bench_cosine_similarity_dense() {
        let mut vec_a = FastMap::default();
        let mut vec_b = FastMap::default();

        for i in 0..100 {
            vec_a.insert(format!("term_{}", i), i as f64);
            vec_b.insert(format!("term_{}", i), (i * 2) as f64);
        }

        let start = std::time::Instant::now();
        for _ in 0..100_000 {
            let _ = cosine_similarity(&vec_a, &vec_b);
        }
        let elapsed = start.elapsed();
        println!(
            "bench_cosine_similarity_dense: {:?} (100k iterations)",
            elapsed
        );
    }

    #[test]
    fn bench_tfidf_computation() {
        let mut docs = Vec::new();

        // Create synthetic corpus
        for doc_id in 0..100 {
            let mut terms = FastMap::default();
            for term_id in 0..50 {
                if term_id % (doc_id + 1) == 0 {
                    terms.insert(format!("term_{}", term_id), (term_id + 1) as u32);
                }
            }
            let length = terms.values().sum();
            docs.push(Document {
                id: doc_id,
                path: format!("doc_{}", doc_id),
                terms,
                length,
            });
        }

        let mut term_df = FastMap::default();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let index = Index { docs, term_df };

        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            for doc in &index.docs {
                let _ = index.vector(doc);
            }
        }
        let elapsed = start.elapsed();
        println!(
            "bench_tfidf_computation: {:?} (1k iterations × 100 docs)",
            elapsed
        );
    }

    #[test]
    fn bench_large_corpus_index() {
        // Simulate realistic corpus: 1000 docs with repeated chunks
        let mut docs = Vec::new();

        let common_chunk: Vec<String> = (0..50).map(|i| format!("common_{}", i)).collect();

        for doc_id in 0..1000 {
            let mut terms = FastMap::default();

            // Add common terms (simulating repeated massive chunks)
            for term in &common_chunk {
                *terms.entry(term.clone()).or_insert(0) += 100;
            }

            // Add document-specific terms
            for term_id in 0..200 {
                if term_id % (doc_id % 10 + 1) == 0 {
                    terms.insert(
                        format!("doc{}_term_{}", doc_id, term_id),
                        (term_id % 20 + 1) as u32,
                    );
                }
            }

            let length = terms.values().sum();
            docs.push(Document {
                id: doc_id,
                path: format!("doc_{}.txt", doc_id),
                terms,
                length,
            });
        }

        let mut term_df = FastMap::default();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let index = Index { docs, term_df };

        let start = std::time::Instant::now();
        let _ = index.vector(&index.docs[0]);
        let elapsed = start.elapsed();
        println!(
            "bench_large_corpus_index: vector computation on 1k doc corpus: {:?}",
            elapsed
        );

        let start = std::time::Instant::now();
        let _ = index.similar_to("doc_0.txt", 10).unwrap();
        let elapsed = start.elapsed();
        println!(
            "bench_large_corpus_index: similarity search across 1k docs: {:?}",
            elapsed
        );
    }

    #[test]
    fn bench_similar_to() {
        let mut docs = Vec::new();

        // Create synthetic corpus with some overlap
        for doc_id in 0..50 {
            let mut terms = FastMap::default();
            for term_id in 0..100 {
                // Create overlap patterns
                if term_id < 20 || term_id % (doc_id + 1) == 0 {
                    terms.insert(
                        format!("term_{}", term_id),
                        ((term_id + doc_id) % 10 + 1) as u32,
                    );
                }
            }
            let length = terms.values().sum();
            docs.push(Document {
                id: doc_id,
                path: format!("doc_{}.txt", doc_id),
                terms,
                length,
            });
        }

        let mut term_df = FastMap::default();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let index = Index { docs, term_df };

        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = index.similar_to("doc_0.txt", 10).unwrap();
        }
        let elapsed = start.elapsed();
        println!("bench_similar_to: {:?} (100 iterations, 50 docs)", elapsed);
    }

    #[test]
    fn bench_top_terms() {
        let mut docs = Vec::new();

        for doc_id in 0..100 {
            let mut terms = FastMap::default();
            for term_id in 0..200 {
                if term_id % (doc_id + 1) == 0 {
                    terms.insert(format!("term_{}", term_id), (term_id % 20 + 1) as u32);
                }
            }
            let length = terms.values().sum();
            docs.push(Document {
                id: doc_id,
                path: format!("doc_{}", doc_id),
                terms,
                length,
            });
        }

        let mut term_df = FastMap::default();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let index = Index { docs, term_df };

        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            let _ = index.top_terms(20);
        }
        let elapsed = start.elapsed();
        println!("bench_top_terms: {:?} (1k iterations, 100 docs)", elapsed);
    }
}
