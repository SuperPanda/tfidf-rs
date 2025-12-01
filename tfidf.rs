#!/usr/bin/env -S rust-script
//! ```cargo
//! [dependencies]
//! rayon = "1.8"
//! bincode = "1.3"
//! serde = { version = "1.0", features = ["derive"] }
//! ```
//!
//! TF-IDF v1 - Immediate similarity queries
//!
//! Usage:
//!   tfidf-quick similar <file> [root] [--ext md,rs]
//!   tfidf-quick terms [root] [--top 20] [--ext md]
//!   tfidf-quick save <root> <index> [--ext md]
//!   tfidf-quick query <index> similar <file>
//!   tfidf-quick query <index> list
//!
//! This is the quick-win script. Use it now while building the full system.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ============================================================================
// TYPES (Level 0)
// ============================================================================

type DocId = usize;
type TermFreq = HashMap<String, u32>;
type DocFreq = HashMap<String, u32>;

// ============================================================================
// ERROR (Level 0)
// ============================================================================

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ============================================================================
// HASHER (Level 1) - Fast hashing for HashMaps
// ============================================================================

use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
struct FastHasher(u64);

impl Hasher for FastHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut s = self.0 ^ 0x9E37_79B9_7F4A_7C15;
        let mut i = 0;
        while i + 8 <= bytes.len() {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[i..i + 8]);
            let mut v = u64::from_le_bytes(chunk);
            v ^= v >> 30;
            v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            v ^= v >> 27;
            v = v.wrapping_mul(0x94D0_49BB_1331_11EB);
            s ^= v;
            s = s.rotate_left(23).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            i += 8;
        }
        for &b in &bytes[i..] {
            let mut v = b as u64;
            v ^= v >> 30;
            v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            s ^= v;
            s = s.rotate_left(27).wrapping_mul(0x94D0_49BB_1331_11EB);
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
type FastSet<T> = HashSet<T, FastBuild>;

// ============================================================================
// TOKENIZER (Level 1)
// ============================================================================

/// Tokenize bytes into lowercase terms, filtering stopwords
fn tokenize(text: &[u8], stopwords: &FastSet<String>) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = Vec::new();

    for &b in text {
        if is_word_byte(b) {
            current.push(b | 0x20); // lowercase ASCII
        } else if !current.is_empty() {
            let word = unsafe { String::from_utf8_unchecked(std::mem::take(&mut current)) };
            if !stopwords.contains(&word) {
                tokens.push(word);
            }
        }
    }

    if !current.is_empty() {
        let word = unsafe { String::from_utf8_unchecked(current) };
        if !stopwords.contains(&word) {
            tokens.push(word);
        }
    }

    tokens
}

#[inline(always)]
fn is_word_byte(b: u8) -> bool {
    let lower = b | 0x20;
    (b'a'..=b'z').contains(&lower) || (b'0'..=b'9').contains(&b) || b == b'_'
}

fn load_stopwords(path: &Path) -> Result<FastSet<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut set = FastSet::default();
    
    for line in reader.lines() {
        let word = line?.trim().to_lowercase();
        if !word.is_empty() {
            set.insert(word);
        }
    }
    
    Ok(set)
}

// ============================================================================
// CORPUS (Level 2)
// ============================================================================

#[derive(Clone, Debug)]
struct Corpus {
    files: Vec<PathBuf>,
}

impl Corpus {
    fn discover(root: &Path) -> Result<Self> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                // Skip hidden by default
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }

        Ok(Corpus { files })
    }

    fn with_extensions(mut self, exts: &[&str]) -> Self {
        self.files.retain(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.iter().any(|ext| e.eq_ignore_ascii_case(ext)))
                .unwrap_or(false)
        });
        self
    }

    fn with_dirs(mut self, dirs: &[&str]) -> Self {
        self.files.retain(|p| {
            let s = p.to_string_lossy();
            dirs.iter().any(|d| s.contains(d))
        });
        self
    }

    fn excluding(mut self, patterns: &[&str]) -> Self {
        self.files.retain(|p| {
            let s = p.to_string_lossy();
            !patterns.iter().any(|pat| glob_match(pat, &s))
        });
        self
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let (mut pi, mut ti) = (0, 0);
    let (mut star_pi, mut star_ti) = (None, 0);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_pi = Some(pi);
            pi += 1;
            star_ti = ti;
        } else if let Some(s) = star_pi {
            pi = s + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }

    pi == p.len()
}

// ============================================================================
// INDEX (Level 3)
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Document {
    id: DocId,
    path: String,
    terms: TermFreq,
    length: u32,
}

#[derive(Serialize, Deserialize)]
struct Index {
    docs: Vec<Document>,
    term_df: DocFreq,
}

impl Index {
    fn build(corpus: &Corpus, stopwords: &FastSet<String>) -> Result<Self> {
        let start = Instant::now();
        if !is_silent() {
            eprint!("🔍 Indexing {} files... ", corpus.files.len());
            io::stderr().flush()?;
        }
        let docs: Vec<Document> = corpus
            .files
            .par_iter()
            .enumerate()
            .filter_map(|(id, path)| {
                let text = fs::read(path).ok()?;
                let tokens = tokenize(&text, stopwords);

                if tokens.is_empty() {
                    return None;
                }

                let mut terms = TermFreq::new();
                for token in tokens {
                    *terms.entry(token).or_insert(0) += 1;
                }

                let length: u32 = terms.values().copied().sum();
                Some(Document {
                    id,
                    path: path.to_string_lossy().into_owned(),
                    terms,
                    length
                })
            })
            .collect();

        // Compute document frequencies
        let mut term_df = DocFreq::new();
        for doc in &docs {
            for term in doc.terms.keys() {
                *term_df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        if !is_silent() {
            eprintln!("✓ {} docs ({:.2}s)", docs.len(), start.elapsed().as_secs_f64());
        }
        Ok(Index { docs, term_df })
    }

    fn save(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }

    fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(bincode::deserialize_from(reader)?)
    }

    // Compute TF-IDF score for a term in a document
    fn tfidf(&self, term: &str, count: u32, doc_len: u32) -> f64 {
        let df = self.term_df.get(term).copied().unwrap_or(1) as f64;
        let num_docs = self.docs.len() as f64;

        // Smooth IDF: ln((D+1)/(df+1))
        let idf = ((num_docs + 1.0) / (df + 1.0)).ln().max(0.0);
        let tf = count as f64 / doc_len.max(1) as f64;

        tf * idf
    }

    // Build TF-IDF vector for a document
    fn doc_vector(&self, doc: &Document) -> FastMap<String, f64> {
        let mut vec = FastMap::default();
        for (term, &count) in &doc.terms {
            let score = self.tfidf(term, count, doc.length);
            vec.insert(term.clone(), score);
        }
        vec
    }
}

// ============================================================================
// QUERY (Level 4)
// ============================================================================

impl Index {
    fn list(&self) -> Result<Vec<String>> {
     Ok(self.docs.iter().map(|doc| { doc.path.clone() }).collect::<Vec<String>>())
    }
    fn similar_to(&self, target: &str, n: usize) -> Result<Vec<(String, f64)>> {
        // Find target document
        let target_doc = self.docs
            .iter()
            .find(|d| d.path == target)
            .ok_or_else(|| format!("Target not found: {}", target))?;

        let target_vec = self.doc_vector(target_doc);

        // Compute similarities in parallel
        let mut scores: Vec<_> = self.docs
            .par_iter()
            .filter(|d| d.id != target_doc.id)
            .map(|doc| {
                let vec = self.doc_vector(doc);
                let sim = cosine_similarity(&target_vec, &vec);
                (doc.path.clone(), sim)
            })
            .filter(|(_, sim)| *sim > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);
        
        Ok(scores)
    }

    fn top_terms(&self, n: usize) -> Vec<(String, f64)> {
        let mut term_scores: FastMap<String, f64> = FastMap::default();

        for doc in &self.docs {
            for (term, &count) in &doc.terms {
                let score = self.tfidf(term, count, doc.length);
                *term_scores.entry(term.clone()).or_insert(0.0) += score;
            }
        }

        let mut scores: Vec<_> = term_scores.into_iter().collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);
        scores
    }
}

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
// CLI (Level 5)
// ============================================================================

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return Ok(());
    }
    match args[0].as_str() {
        "similar" => cmd_similar(&args[1..])?,
        "terms" => cmd_terms(&args[1..])?,
        "save" => cmd_save(&args[1..])?,
        "query" => cmd_query(&args[1..])?,
        _ => {
            eprintln!("Unknown command: {}\n", args[0]);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn cmd_similar(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Usage: tfidf-quick similar <file> [root] [--ext ext1,ext2]");
        std::process::exit(1);
    }

    let target = &args[0];
    let root = if args.len() > 1 && !args[1].starts_with("--") {
        Path::new(&args[1])
    } else {
        Path::new(".")
    };

    let (extensions, dirs, excludes) = parse_filters(args);
    let stopwords = load_stopwords_if_exists()?;

    let mut corpus = Corpus::discover(root)?;
    if !extensions.is_empty() {
        let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
        corpus = corpus.with_extensions(&ext_refs);
    }
    if !dirs.is_empty() {
        let dir_refs: Vec<&str> = dirs.iter().map(|s| s.as_str()).collect();
        corpus = corpus.with_dirs(&dir_refs);    
    }
    if !excludes.is_empty() {
        let ex_refs: Vec<&str> = excludes.iter().map(|s| s.as_str()).collect();
        corpus = corpus.excluding(&ex_refs);
    }


    let index = Index::build(&corpus, &stopwords)?;
    let similar = index.similar_to(target, 10)?;

    if !is_silent() {
        println!("\n🔍 Similar to: {}\n", target);
        println!("{:<50} | {:>10}", "Path", "Similarity");
        println!("{}", "-".repeat(70));
    }

    for (path, score) in similar {
        // Shorten path if too long
        let display_path = if path.len() > 50 {
            format!("...{}", &path[path.len()-47..])
        } else {
            path
        };
        println!("{:<50} | {:>10.4}", display_path, score);
    }

    Ok(())
}

fn cmd_terms(args: &[String]) -> Result<()> {
    let root = if !args.is_empty() && !args[0].starts_with("--") {
        Path::new(&args[0])
    } else {
        Path::new(".")
    };

    let (extensions, _, _) = parse_filters(args);
    let top_n = parse_flag_value(args, "--top").unwrap_or(20);
    let stopwords = load_stopwords_if_exists()?;

    let mut corpus = Corpus::discover(root)?;
    if !extensions.is_empty() {
        let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
        corpus = corpus.with_extensions(&ext_refs);
    }

    let index = Index::build(&corpus, &stopwords)?;
    let terms = index.top_terms(top_n);

    if !is_silent() {
        println!("\n📊 Top {} Terms\n", top_n);
        println!("{:<30} | {:>12}", "Term", "Score");
        println!("{}", "-".repeat(50));
    }

    for (term, score) in terms {
        println!("{:<30} | {:>12.4}", term, score);
    }

    Ok(())
}

fn cmd_save(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        eprintln!("Usage: tfidf-quick save <root> <index-file> [--ext ext1,ext2]");
        std::process::exit(1);
    }

    let root = Path::new(&args[0]);
    let out_path = Path::new(&args[1]);
    let (extensions, _, _) = parse_filters(args);
    let stopwords = load_stopwords_if_exists()?;

    let mut corpus = Corpus::discover(root)?;
    if !extensions.is_empty() {
        let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
        corpus = corpus.with_extensions(&ext_refs);
    }

    let index = Index::build(&corpus, &stopwords)?;
    index.save(out_path)?;

    let size_mb = fs::metadata(out_path)?.len() as f64 / (1024.0 * 1024.0);
    eprintln!("💾 Saved {} docs → {} ({:.2} MB)", 
              index.docs.len(), 
              out_path.display(),
              size_mb);

    Ok(())
}

fn cmd_query(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        eprintln!("Usage: tfidf-quick query <index-file> similar <file>");
        eprintln!("       tfidf-quick query <index-file> terms [--top N]");
        std::process::exit(1);
    }

    let index_path = Path::new(&args[0]);
    let subcmd = &args[1];


    if !is_silent() {
        eprint!("📂 Loading index... ");
        io::stderr().flush()?;
    }
    let index = Index::load(index_path)?;
    if !is_silent() {
       eprintln!("✓ {} docs", index.docs.len());
    }
    match subcmd.as_str() {
        "list" => {
            let paths = index.list()?;
            for path in paths {
                println!("{}",path);
            }
        },
        "similar" => {
            if args.len() < 3 {
                eprintln!("Usage: tfidf-quick query <index-file> similar <file>");
                std::process::exit(1);
            }
            let target = &args[2];
            let similar = index.similar_to(target, 10)?;
            if !is_silent() {
                println!("\n🔍 Similar to: {}\n", target);
                println!("{:<50} | {:>10}", "Path", "Similarity");
                println!("{}", "-".repeat(70));
            }
            for (path, score) in similar {
                let display_path = if path.len() > 50 {
                    format!("...{}", &path[path.len()-47..])
                } else {
                    path
                };
                println!("{:<50} | {:>10.4}", display_path, score);
            }
        }
        "terms" => {
            let top_n = parse_flag_value(&args[2..], "--top").unwrap_or(20);
            let terms = index.top_terms(top_n);

            println!("\n📊 Top {} Terms\n", top_n);
            println!("{:<30} | {:>12}", "Term", "Score");
            println!("{}", "-".repeat(50));

            for (term, score) in terms {
                println!("{:<30} | {:>12.4}", term, score);
            }
        }
        _ => {
            eprintln!("Unknown query command: {}", subcmd);
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Utilities
// ============================================================================

fn parse_filters(args: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut extensions = Vec::new();
    let mut dirs = Vec::new();
    let mut excludes = Vec::new();

    for i in 0..args.len() {
        if args[i] == "--ext" && i + 1 < args.len() {
            extensions.extend(args[i + 1].split(',').map(|s| s.trim().to_string()));
        } else if args[i] == "--dir" && i + 1 < args.len() {
            dirs.extend(args[i + 1].split(',').map(|s| s.trim().to_string()));
        } else if args[i] == "--exclude" && i + 1 < args.len() {
            excludes.extend(args[i + 1].split(',').map(|s| s.trim().to_string()));
        }
    }

    (extensions, dirs, excludes)
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<usize> {
    args.iter()
        .position(|s| s == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

fn load_stopwords_if_exists() -> Result<FastSet<String>> {
    // Try XDG config location
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg_config).join("tfidf/stopwords.txt");
        if path.exists() {
            return load_stopwords(&path);
        }
    }
    
    // Try ~/.config
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".config/tfidf/stopwords.txt");
        if path.exists() {
            return load_stopwords(&path);
        }
    }
    
    // No stopwords file found, return empty set
    Ok(FastSet::default())
}

fn is_silent() -> bool {

    const SILENT_MODE_KEY: &str="TFIDF_SILENT";

    let is_silent: bool = match std::env::var(SILENT_MODE_KEY) {
        Ok(val) => {
            if val.as_str() == "1" {
                true
            } else {
                false 
            }

        }
        _ => false        
    };
    is_silent
}

fn print_help() {
    eprintln!(r#"
TF-IDF Quick Win - Document similarity and analysis

USAGE:
    tfidf-quick <COMMAND> [OPTIONS]

COMMANDS:
    similar <file> [root]       Find documents similar to <file>
    terms [root]                Show top terms in corpus
    save <root> <index>         Save index for repeated queries
    query <index> similar <f>   Query saved index for similar docs
    query <index> terms         Query saved index for top terms
    query <index> list          Shows files within index

OPTIONS:
    --ext <ext1,ext2>   Include only these extensions
    --top <N>           Show top N results (default: 10 for similar, 20 for terms)

EXAMPLES:
    # Find similar documents
    tfidf-quick similar notes/compilers.md ~/notes --ext md

    # Show top terms
    tfidf-quick terms ~/notes --ext md --top 50

    # Save index for repeated queries
    tfidf-quick save ~/notes notes.idx --ext md
    tfidf-quick query notes.idx similar compilers.md

CONFIGURATION:
    Stopwords: ~/.config/tfidf/stopwords.txt (one word per line)

ENVIRONMENT VARIABLES:
    TFIDF_SILENT:   Hides table headers and formatting (for machine parsable output) if set to "1"

NOTE: This is the quick-win script. Full system with clap, man pages,
      and advanced features coming in Phase 1.
"#);
}
