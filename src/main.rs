// A composable TF-IDF index for pipe-friendly text analysis.
//
// Usage:
//   tfidf save [--index <path>] [PATH...] [--]
//   tfidf list [--index <path>]
//   tfidf terms [--index <path>] [--top N]
//   tfidf similar [--index <path>] <file> [--top N]
//   tfidf search [--index <path>] [query] [--top N]
//   tfidf {-v|--version|version}
//
// The older `tfidf query ...` spelling remains a compatibility alias.
//
// Index resolution:
//   --index > TFIDF_INDEX > resolved data directory/tfidf.index
//
// Every positional save argument is an input path. A trailing -- reads
// additional input paths from stdin, one per line.

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
// Fast hasher
// ============================================================================

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

/// Buffered stdout with graceful handling of a downstream closed pipe.
struct PipeWriter {
    writer: BufWriter<io::Stdout>,
}

impl PipeWriter {
    fn new() -> Self {
        Self {
            writer: BufWriter::new(io::stdout()),
        }
    }

    fn writeln(&mut self, s: &str) -> Result<()> {
        match writeln!(self.writer, "{s}") {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::BrokenPipe => {
                std::process::exit(0);
            }
            Err(error) => Err(error.into()),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self.writer.flush() {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::BrokenPipe => {
                std::process::exit(0);
            }
            Err(error) => Err(error.into()),
        }
    }
}

// ============================================================================
// Tokenization and configuration
// ============================================================================

#[inline]
fn is_word_byte(b: u8) -> bool {
    matches!(b | 0x20, b'a'..=b'z') || b.is_ascii_digit() || b == b'_'
}

/// Extract lowercase ASCII word tokens, filtering configured stopwords.
fn tokenize(text: &[u8], stopwords: &FastMap<String, ()>) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = Vec::new();

    for &b in text {
        if is_word_byte(b) {
            current.push(b | 0x20);
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

fn app_name() -> String {
    env::var("TFIDF_APPNAME")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_APP_NAME.to_string())
}

/// Load the first existing stopwords file:
///   1. $XDG_CONFIG_HOME/$TFIDF_APPNAME/stopwords.txt
///   2. $HOME/.config/$TFIDF_APPNAME/stopwords.txt
fn load_stopwords() -> Result<FastMap<String, ()>> {
    let app = app_name();

    let paths = [
        env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|path| PathBuf::from(path).join(&app).join("stopwords.txt")),
        env::var("HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|home| {
                PathBuf::from(home)
                    .join(".config")
                    .join(&app)
                    .join("stopwords.txt")
            }),
    ];

    for path in paths.into_iter().flatten() {
        if path.exists() {
            let reader = BufReader::new(File::open(path)?);
            let mut stopwords = FastMap::default();

            for line in reader.lines() {
                let word = line?.trim().to_lowercase();
                if !word.is_empty() {
                    stopwords.insert(word, ());
                }
            }

            return Ok(stopwords);
        }
    }

    Ok(FastMap::default())
}

// ============================================================================
// Paths
// ============================================================================

fn cwd_or_dot() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    env_path("HOME")
}

/// Resolve the data directory:
///   1. $TFIDF_DATA_DIR
///   2. $XDG_DATA_HOME/$TFIDF_APPNAME
///   3. $HOME/.local/share/$TFIDF_APPNAME
///   4. ./$TFIDF_APPNAME
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

fn xdg_index_path() -> PathBuf {
    tfidf_data_dir().join(DEFAULT_INDEX_FILE)
}

/// Resolve --index > TFIDF_INDEX > data-directory default.
fn resolve_index_path(explicit: Option<&str>) -> PathBuf {
    if let Some(path) = explicit {
        return PathBuf::from(path);
    }

    if let Some(path) = env_path("TFIDF_INDEX") {
        return path;
    }

    xdg_index_path()
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
// Corpus discovery
// ============================================================================

struct Corpus {
    files: Vec<PathBuf>,
}

impl Corpus {
    /// Recursively discover files, silently skipping nonexistent input paths.
    fn from_paths<I>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut files = Vec::new();

        for path in paths {
            if path.is_dir() {
                Self::walk_dir(&path, &mut files)?;
            } else if path.is_file() {
                files.push(path);
            }
        }

        Ok(Self { files })
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

#[derive(Clone, Serialize, Deserialize)]
struct Document {
    id: u32,
    path: String,
    terms: TermFreq,
    length: u32,
}

/// term_df[t] is the number of documents containing t.
///
/// Field layout and serialization remain unchanged.
#[derive(Serialize, Deserialize)]
struct Index {
    docs: Vec<Document>,
    term_df: FastMap<String, u32>,
}

impl Index {
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

        let mut term_df = FastMap::default();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        Ok(Self { docs, term_df })
    }

    fn save(&self, path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }

    fn load(path: &Path) -> Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        Ok(bincode::deserialize_from(reader)?)
    }

    /// TF(t,d) * ln((D + 1) / (df(t) + 1)).
    fn tfidf(&self, term: &str, count: u32, doc_len: u32) -> f64 {
        let df = self.term_df.get(term).copied().unwrap_or(1) as f64;
        let num_docs = self.docs.len().max(1) as f64;

        let idf = ((num_docs + 1.0) / (df + 1.0)).ln().max(0.0);
        let tf = count as f64 / doc_len.max(1) as f64;

        tf * idf
    }

    fn vector(&self, doc: &Document) -> FastMap<String, f64> {
        doc.terms
            .iter()
            .map(|(term, &count)| (term.clone(), self.tfidf(term, count, doc.length)))
            .collect()
    }

    fn list_paths(&self) -> impl Iterator<Item = &str> {
        self.docs.iter().map(|doc| doc.path.as_str())
    }

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

    fn similar_to(&self, target: &str, n: usize) -> Result<Vec<(String, f64)>> {
        let target_doc = self
            .docs
            .iter()
            .find(|doc| doc.path == target)
            .ok_or_else(|| format!("target not found: {target}"))?;

        let target_vec = self.vector(target_doc);

        let mut scores: Vec<_> = self
            .docs
            .par_iter()
            .filter(|doc| doc.id != target_doc.id)
            .map(|doc| {
                let vector = self.vector(doc);
                let score = cosine_similarity(&target_vec, &vector);
                (doc.path.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);

        Ok(scores)
    }

    fn search(&self, query: &str, n: usize, stopwords: &FastMap<String, ()>) -> Vec<(String, f64)> {
        let tokens = tokenize(query.as_bytes(), stopwords);

        if tokens.is_empty() {
            return Vec::new();
        }

        let mut query_terms: TermFreq = FastMap::default();
        for token in tokens {
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
                let vector = self.vector(doc);
                let score = cosine_similarity(&query_vec, &vector);
                (doc.path.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);

        scores
    }
}

/// Cosine similarity of two sparse vectors.
///
/// This is a similarity measure, not itself a distance metric.
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

    // Top-level version handling must precede all configuration loading,
    // index resolution, and application filesystem access.
    if args.first().is_some_and(|arg| is_version_command(arg)) {
        return print_version();
    }

    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help") {
        print_usage();
        return Ok(());
    }

    let command = args.remove(0);

    match command.as_str() {
        "save" => cmd_save(args),
        "list" | "terms" | "similar" | "search" => cmd_read(&command, args),
        "query" => cmd_query(args),
        _ => {
            print_usage();
            Err(format!("unknown command: {command}").into())
        }
    }
}

fn is_version_command(command: &str) -> bool {
    matches!(command, "-v" | "--version" | "version")
}

fn print_version() -> Result<()> {
    let mut out = PipeWriter::new();
    out.writeln(&format!("tfidf {}", env!("CARGO_PKG_VERSION")))?;
    out.flush()
}

/// Remove --index <path>, leaving all other arguments in order.
///
/// As before, the last --index wins. A standalone -- terminates option
/// scanning; arguments after it are not interpreted as options.
fn take_index_flag(args: &mut Vec<String>) -> Result<Option<String>> {
    let mut index_arg = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--" => break,
            "--index" => {
                let value = args
                    .get(i + 1)
                    .filter(|value| !value.is_empty() && !value.starts_with("--"))
                    .ok_or("--index requires a path")?;

                index_arg = Some(value.clone());
                args.drain(i..i + 2);
            }
            _ => i += 1,
        }
    }

    Ok(index_arg)
}

/// Every positional argument is an input path, irrespective of extension
/// or existence. Only --index or normal resolution selects the destination.
fn cmd_save(mut args: Vec<String>) -> Result<()> {
    let explicit_index = take_index_flag(&mut args)?;
    let index_path = resolve_index_path(explicit_index.as_deref());

    let mut paths = Vec::new();
    let mut read_stdin = false;

    for arg in args {
        if arg == "--" {
            read_stdin = true;
            break;
        }

        paths.push(PathBuf::from(arg));
    }

    if read_stdin {
        for line in io::stdin().lock().lines() {
            let line = line?;
            let path = line.trim();

            if !path.is_empty() {
                paths.push(PathBuf::from(path));
            }
        }
    }

    if paths.is_empty() {
        return Err("tfidf save: no input paths\n\
             usage: tfidf save [--index <path>] [PATH...] [--]"
            .into());
    }

    let stopwords = load_stopwords()?;
    let corpus = Corpus::from_paths(paths)?;
    let index = Index::build(&corpus, &stopwords)?;

    index.save(&index_path)
}

/// Compatibility entry point for `tfidf query ...`.
///
/// Extracting --index here preserves support for placing it before the
/// subcommand, as in `tfidf query --index docs.index list`.
fn cmd_query(mut args: Vec<String>) -> Result<()> {
    let index_arg = take_index_flag(&mut args)?;

    if args.is_empty() {
        print_usage();
        return Err("tfidf query: a subcommand is required".into());
    }

    let command = args.remove(0);
    execute_read(&command, args, index_arg)
}

/// Entry point for direct read commands.
fn cmd_read(command: &str, mut args: Vec<String>) -> Result<()> {
    let index_arg = take_index_flag(&mut args)?;
    execute_read(command, args, index_arg)
}

fn execute_read(command: &str, args: Vec<String>, index_arg: Option<String>) -> Result<()> {
    if !matches!(command, "list" | "terms" | "similar" | "search") {
        print_usage();
        return Err(format!("unknown query command: {command}").into());
    }

    let index_path = resolve_index_path(index_arg.as_deref());

    if !index_path.exists() {
        return Err(format!("index not found: {}", index_path.display()).into());
    }

    let index = Index::load(&index_path)?;

    match command {
        "list" => cmd_list(index),
        "terms" => cmd_terms(index, args),
        "similar" => cmd_similar(index, args),
        "search" => cmd_search(index, args),
        _ => unreachable!("read command validated above"),
    }
}

/// Preserve the existing --top parsing behavior.
fn parse_top(args: &[String], default: usize) -> usize {
    args.windows(2)
        .find(|pair| pair[0] == "--top")
        .and_then(|pair| pair[1].parse().ok())
        .unwrap_or(default)
}

fn cmd_list(index: Index) -> Result<()> {
    let mut out = PipeWriter::new();

    for path in index.list_paths() {
        out.writeln(path)?;
    }

    out.flush()
}

fn cmd_terms(index: Index, args: Vec<String>) -> Result<()> {
    let n = parse_top(&args, 20);
    let mut out = PipeWriter::new();

    for (term, score) in index.top_terms(n) {
        out.writeln(&format!("{term}\t{score:.6}"))?;
    }

    out.flush()
}

fn cmd_similar(index: Index, args: Vec<String>) -> Result<()> {
    if args.is_empty() {
        return Err("usage: tfidf similar [--index <path>] <file> [--top N]".into());
    }

    let target = &args[0];
    let n = parse_top(&args[1..], 10);
    let mut out = PipeWriter::new();

    for (path, score) in index.similar_to(target, n)? {
        out.writeln(&format!("{path}\t{score:.6}"))?;
    }

    out.flush()
}

fn cmd_search(index: Index, args: Vec<String>) -> Result<()> {
    let n = parse_top(&args, 10);
    let stopwords = load_stopwords()?;

    // Preserve the existing single-argument query interface. Quote
    // multiword queries in the shell, or provide the query on stdin.
    let query = if args.is_empty() || args[0] == "--top" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else {
        args[0].clone()
    };

    if query.is_empty() {
        return Err("tfidf search: empty query".into());
    }

    let mut out = PipeWriter::new();

    for (path, score) in index.search(&query, n, &stopwords) {
        out.writeln(&format!("{path}\t{score:.6}"))?;
    }

    out.flush()
}

fn print_usage() {
    eprintln!(
        "\
tfidf - composable document similarity

USAGE
  tfidf save [--index <path>] [PATH...] [--]
  tfidf list [--index <path>]
  tfidf terms [--index <path>] [--top N]
  tfidf similar [--index <path>] <file> [--top N]
  tfidf search [--index <path>] [query] [--top N]
  tfidf -v
  tfidf --version
  tfidf version

SAVE
  Build an index from files and directories.
  Every positional argument is an input path.
  Select the destination with --index or the index resolution cascade.
  The destination and its parent directories are created as needed.
  Use a trailing -- to read additional paths from stdin, one per line.
  Nonexistent input paths are silently skipped.

READ COMMANDS
  list                     List all indexed documents
  terms [--top N]           Show top N terms (default: 20)
  similar <file> [--top N]  Find similar documents (default: 10)
  search [query] [--top N]  Search by query string (default: 10)
                           Reads from stdin if query is not provided

  The older 'tfidf query [--index <path>] <command>' form is also accepted.

VERSION
  -v, --version, version   Print the version and exit
                           Recognized only as top-level commands

INDEX RESOLUTION
  1. --index flag
  2. TFIDF_INDEX environment variable
  3. Resolved data directory/tfidf.index

DATA DIRECTORY RESOLUTION
  1. $TFIDF_DATA_DIR
  2. $XDG_DATA_HOME/$TFIDF_APPNAME
  3. $HOME/.local/share/$TFIDF_APPNAME
  4. ./$TFIDF_APPNAME

  Empty environment values are ignored.
  TFIDF_APPNAME defaults to tfidf.

CONFIGURATION
  Stopwords are loaded from the first existing file:
    $XDG_CONFIG_HOME/$TFIDF_APPNAME/stopwords.txt
    $HOME/.config/$TFIDF_APPNAME/stopwords.txt

OUTPUT
  Tab-delimited for composition with cut, sort, xargs, etc.
  Closed downstream pipes exit successfully.

EXAMPLES
  # Save using the default resolved index path
  tfidf save src/ notes/

  # Mix positional inputs with paths supplied on stdin
  find notes -type f | tfidf save src/ --

  # Select the destination explicitly
  tfidf save --index docs.index src/ notes/

  # Query
  tfidf list | head -5
  tfidf terms --top 100 | sort -k2,2nr
  tfidf similar main.rs | cut -f1 | xargs cat
  tfidf search \"error handling rust\" --top 20
  echo \"memory management\" | tfidf search --top 10

  # Use separate configuration and data
  TFIDF_APPNAME=work tfidf save ~/work/docs
"
    );
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn version_spellings() {
        for command in ["-v", "--version", "version"] {
            assert!(is_version_command(command));
        }

        for command in ["save", "search", "query", "-V", "file.index"] {
            assert!(!is_version_command(command));
        }
    }

    #[test]
    fn index_flag_is_removed_without_consuming_inputs() {
        let mut args = strings(&[
            "existing.index",
            "--index",
            "destination.index",
            "other.txt",
        ]);

        let explicit = take_index_flag(&mut args).unwrap();

        assert_eq!(explicit.as_deref(), Some("destination.index"));
        assert_eq!(args, strings(&["existing.index", "other.txt"]));
    }

    #[test]
    fn positional_index_names_remain_inputs() {
        let mut args = strings(&["existing.index", "missing.index", "other.txt"]);

        assert_eq!(take_index_flag(&mut args).unwrap(), None);
        assert_eq!(
            args,
            strings(&["existing.index", "missing.index", "other.txt"])
        );
    }

    #[test]
    fn index_flag_missing_value_fails() {
        for values in [
            vec!["--index"],
            vec!["input.txt", "--index"],
            vec!["--index", "--"],
            vec!["--index", "--top", "5"],
            vec!["--index", ""],
        ] {
            let error = take_index_flag(&mut strings(&values)).unwrap_err();
            assert_eq!(error.to_string(), "--index requires a path");
        }
    }

    #[test]
    fn index_flag_stops_at_stdin_marker() {
        let mut args = strings(&["input.txt", "--", "--index", "ignored.index"]);

        assert_eq!(take_index_flag(&mut args).unwrap(), None);
        assert_eq!(
            args,
            strings(&["input.txt", "--", "--index", "ignored.index"])
        );
    }

    #[test]
    fn last_index_flag_wins() {
        let mut args = strings(&[
            "--index",
            "first.index",
            "input.txt",
            "--index",
            "second.index",
        ]);

        assert_eq!(
            take_index_flag(&mut args).unwrap().as_deref(),
            Some("second.index")
        );
        assert_eq!(args, strings(&["input.txt"]));
    }

    #[test]
    fn explicit_index_resolution_does_not_need_environment() {
        assert_eq!(
            resolve_index_path(Some("chosen.index")),
            PathBuf::from("chosen.index")
        );
    }

    #[test]
    fn word_bytes() {
        for byte in [b'a', b'Z', b'5', b'_'] {
            assert!(is_word_byte(byte));
        }

        for byte in [b' ', b'.', b'-'] {
            assert!(!is_word_byte(byte));
        }
    }

    #[test]
    fn tokenization() {
        let stopwords = FastMap::default();

        assert_eq!(
            tokenize(b"Hello, World! test-case var_2", &stopwords),
            strings(&["hello", "world", "test", "case", "var_2"])
        );
    }

    #[test]
    fn tokenization_with_stopwords() {
        let mut stopwords = FastMap::default();
        stopwords.insert("the".to_string(), ());

        assert_eq!(
            tokenize(b"The quick brown fox", &stopwords),
            strings(&["quick", "brown", "fox"])
        );
    }

    #[test]
    fn parse_top_defaults_and_explicit_values() {
        assert_eq!(parse_top(&[], 42), 42);
        assert_eq!(parse_top(&strings(&["--top", "100"]), 42), 100);
        assert_eq!(parse_top(&strings(&["--top", "invalid"]), 42), 42);
        assert_eq!(parse_top(&strings(&["--top"]), 42), 42);
    }

    #[test]
    fn cosine_identical() {
        let mut vector = FastMap::default();
        vector.insert("term1".to_string(), 1.0);
        vector.insert("term2".to_string(), 2.0);

        assert!((cosine_similarity(&vector, &vector) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_orthogonal() {
        let mut a = FastMap::default();
        a.insert("term1".to_string(), 1.0);

        let mut b = FastMap::default();
        b.insert("term2".to_string(), 1.0);

        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_partial_overlap() {
        let mut a = FastMap::default();
        a.insert("common".to_string(), 1.0);
        a.insert("unique_a".to_string(), 1.0);

        let mut b = FastMap::default();
        b.insert("common".to_string(), 1.0);
        b.insert("unique_b".to_string(), 1.0);

        assert!((cosine_similarity(&a, &b) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn cosine_empty() {
        let empty = FastMap::default();
        let mut nonempty = FastMap::default();
        nonempty.insert("term".to_string(), 1.0);

        assert_eq!(cosine_similarity(&empty, &nonempty), 0.0);
        assert_eq!(cosine_similarity(&empty, &empty), 0.0);
    }

    #[test]
    fn tfidf_rare_term_scores_above_universal_term() {
        let mut first_terms = FastMap::default();
        first_terms.insert("rare".to_string(), 1);
        first_terms.insert("common".to_string(), 2);

        let mut second_terms = FastMap::default();
        second_terms.insert("common".to_string(), 1);

        let mut term_df = FastMap::default();
        term_df.insert("rare".to_string(), 1);
        term_df.insert("common".to_string(), 2);

        let index = Index {
            docs: vec![
                Document {
                    id: 0,
                    path: "first.txt".to_string(),
                    terms: first_terms,
                    length: 3,
                },
                Document {
                    id: 1,
                    path: "second.txt".to_string(),
                    terms: second_terms,
                    length: 1,
                },
            ],
            term_df,
        };

        assert!(index.tfidf("rare", 1, 3) > index.tfidf("common", 2, 3));
        assert_eq!(
            index.list_paths().collect::<Vec<_>>(),
            vec!["first.txt", "second.txt"]
        );
    }
}
