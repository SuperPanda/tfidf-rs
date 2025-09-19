#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! csv = "1.3"
//! serde = { version = "1.0", features = ["derive"] }
//! ```
//!
//! `explore_tfidf.rs` — Fixed and enhanced TF-IDF data explorer.
//! Now supports slicing, compact output, and similarity queries.
//!
//! Usage:
//!   ./explore_tfidf.rs <tfidf_tsv_file> [--slice "ext:rs dir:src"] [--top-terms 10] [--similar-to PATH]

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use csv::ReaderBuilder;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)] // ✅ FIXED: Added Clone
struct TfidfRecord {
    term: String,
    file_path: String,
    term_freq: u32,
    doc_length: u64,
    doc_freq: u64,
    tf: String,
    idf: String,
    score: u128,
    formula: String,
}

// ✅ FIXED: Removed unused imports
// use std::io::{self, BufReader}; // 'self' was unused
// use itertools::Itertools; // Unused, removed

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_tfidf_tsv_file> [--slice QUERY] [--top-terms N] [--similar-to PATH]", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("📊 Exploring TF-IDF Data: {}", file_path);
    println!("{}", "=".repeat(80));

    // Parse optional flags
    let mut slice_query = None;
    let mut top_terms = None;
    let mut similar_to = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--slice" => {
                if i + 1 < args.len() {
                    slice_query = Some(parse_slice_query(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("--slice requires a query string");
                    std::process::exit(1);
                }
            }
            "--top-terms" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<usize>() {
                        top_terms = Some(n);
                    }
                    i += 2;
                } else {
                    eprintln!("--top-terms requires a number");
                    std::process::exit(1);
                }
            }
            "--similar-to" => {
                if i + 1 < args.len() {
                    similar_to = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("--similar-to requires a file path");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown flag: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut csv_reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_reader(reader);

    let mut records: Vec<TfidfRecord> = Vec::new();

    for result in csv_reader.deserialize() {
        let record: TfidfRecord = result?;
        records.push(record.clone());
    }

    if records.is_empty() {
        println!("No data found in the file.");
        return Ok(());
    }

    // Apply slice filter if specified
    if let Some(slice) = slice_query {
        records.retain(|r| slice.matches(&Path::new(&r.file_path)));
    }

    if records.is_empty() {
        println!("No records match the slice criteria.");
        return Ok(());
    }

    // Handle query modes
    if let Some(n) = top_terms {
        return show_top_terms(&records, n);
    }

    if let Some(target_path) = similar_to {
        return find_similar_files(&records, &target_path);
    }

    // Default: Show summary
    show_summary(&records)?;

    Ok(())
}

// ----------------------------
// Slice Query Parser & Matcher
// ----------------------------

#[derive(Debug, Default)]
struct SliceQuery {
    include_globs: Vec<String>,
    exclude_globs: Vec<String>,
    extensions: Vec<String>,
    dirs: Vec<String>,
}

impl SliceQuery {
    fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();

        // Check exclude globs first
        if self.exclude_globs.iter().any(|g| matches_glob(g, &s)) {
            return false;
        }

        // Check include globs (if any)
        if !self.include_globs.is_empty() && !self.include_globs.iter().any(|g| matches_glob(g, &s)) {
            return false;
        }

        // Check extensions
        if !self.extensions.is_empty() {
            let has_matching_ext = self.extensions.iter().any(|ext| {
                path.extension()
                    .map_or(false, |e| e.to_string_lossy().eq_ignore_ascii_case(ext))
            });
            if !has_matching_ext {
                return false;
            }
        }

        // Check dirs
        if !self.dirs.is_empty() {
            let has_matching_dir = self.dirs.iter().any(|dir| {
                path.starts_with(dir) || path.parent().map_or(false, |p| p.starts_with(dir))
            });
            if !has_matching_dir {
                return false;
            }
        }

        true
    }
}

fn parse_slice_query(query: &str) -> SliceQuery {
    let mut slice = SliceQuery::default();
    for part in query.split_whitespace() {
        if part.starts_with('!') {
            slice.exclude_globs.push(part[1..].to_string());
        } else if part.starts_with("ext:") {
            slice.extensions.extend(part[4..].split(',').map(|s| s.to_string()));
        } else if part.starts_with("dir:") {
            slice.dirs.extend(part[4..].split(',').map(|s| s.to_string()));
        } else {
            slice.include_globs.push(part.to_string());
        }
    }
    slice
}

// Simple glob matching (same as tfidf-calc.rs)
fn matches_glob(glob: &str, text: &str) -> bool {
    let g = glob.as_bytes();
    let t = text.as_bytes();
    let (mut gi, mut ti) = (0, 0);
    let (mut star_gi, mut star_ti) = (None, 0);

    while ti < t.len() {
        if gi < g.len() && (g[gi] == b'?' || g[gi] == t[ti]) {
            gi += 1;
            ti += 1;
        } else if gi < g.len() && g[gi] == b'*' {
            star_gi = Some(gi);
            gi += 1;
            star_ti = ti;
        } else if let Some(s) = star_gi {
            gi = s + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while gi < g.len() && g[gi] == b'*' {
        gi += 1;
    }

    gi == g.len()
}

// ----------------------------
// Analysis Functions
// ----------------------------

fn show_summary(records: &[TfidfRecord]) -> Result<(), Box<dyn std::error::Error>> {
    let total_terms: usize = records.iter().map(|r| &r.term).collect::<HashSet<_>>().len();
    let total_docs: usize = records.iter().map(|r| &r.file_path).collect::<HashSet<_>>().len();
    let total_score: u128 = records.iter().map(|r| r.score).sum();

    println!("📈 GLOBAL SUMMARY");
    println!("Total Unique Terms: {}", total_terms);
    println!("Total Documents: {}", total_docs);
    println!("Total Term-Document Pairs: {}", records.len());
    println!("Aggregate TF-IDF Score: {}", total_score);
    println!();

    // Top 10 terms by total score
    let mut term_scores: HashMap<&String, u128> = HashMap::new();
    for record in records {
        *term_scores.entry(&record.term).or_insert(0) += record.score;
    }

    let mut top_terms: Vec<_> = term_scores.into_iter().collect();
    top_terms.sort_by(|a, b| b.1.cmp(&a.1));

    println!("🏆 TOP 10 TERMS BY TOTAL TF-IDF SCORE");
    for (i, (term, score)) in top_terms.iter().take(10).enumerate() {
        println!("{:2}. {:<20} | Score: {}", i + 1, term, score);
    }
    println!();

    // Top 10 documents by total score
    let mut doc_scores: HashMap<&String, u128> = HashMap::new();
    for record in records {
        *doc_scores.entry(&record.file_path).or_insert(0) += record.score;
    }

    let mut top_docs: Vec<_> = doc_scores.into_iter().collect();
    top_docs.sort_by(|a, b| b.1.cmp(&a.1));

    println!("📂 TOP 10 DOCUMENTS BY TOTAL TF-IDF SCORE");
    for (i, (path, score)) in top_docs.iter().take(10).enumerate() {
        println!("{:2}. {:<40} | Score: {}", i + 1, path.split('/').last().unwrap_or(path), score);
    }

    Ok(())
}

fn show_top_terms(records: &[TfidfRecord], n: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut term_scores: HashMap<&String, u128> = HashMap::new();
    for record in records {
        *term_scores.entry(&record.term).or_insert(0) += record.score;
    }

    let mut top_terms: Vec<_> = term_scores.into_iter().collect();
    top_terms.sort_by(|a, b| b.1.cmp(&a.1));

    println!("🎯 TOP {} TERMS IN SLICE", n);
    println!("{:<20} | {:>10}", "Term", "Total Score");
    println!("{}", "-".repeat(40));
    for (term, score) in top_terms.iter().take(n) {
        println!("{:<20} | {:>10}", term, score);
    }

    Ok(())
}

fn find_similar_files(records: &[TfidfRecord], target_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Build document vectors: doc_path -> (term -> score)
    let mut doc_vectors: HashMap<&String, HashMap<&String, u128>> = HashMap::new();
    for record in records {
        let doc_map = doc_vectors.entry(&record.file_path).or_insert_with(HashMap::new);
        doc_map.insert(&record.term, record.score);
    }

    if !doc_vectors.contains_key(&target_path.to_string()) {
        eprintln!("Target file not found in dataset: {}", target_path);
        return Ok(());
    }

    let target_vector = doc_vectors.get(&target_path.to_string()).unwrap();

    // Compute cosine similarity
    let mut similarities: Vec<(&String, f64)> = Vec::new();

    for (doc_path, vector) in &doc_vectors {
        if *doc_path == target_path {
            continue;
        }

        let similarity = cosine_similarity(target_vector, vector);
        similarities.push((doc_path, similarity));
    }

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("🔍 FILES MOST SIMILAR TO: {}", target_path);
    println!("{:<50} | {:>8}", "File Path", "Similarity");
    println!("{}", "-".repeat(70));
    for (path, sim) in similarities.iter().take(10) {
        println!("{:<50} | {:>8.4}", path, sim);
    }

    Ok(())
}

fn cosine_similarity(a: &HashMap<&String, u128>, b: &HashMap<&String, u128>) -> f64 {
    let mut dot_product = 0u128;
    let mut norm_a = 0u128;
    let mut norm_b = 0u128;

    for (term, &score_a) in a {
        norm_a += score_a * score_a;
        if let Some(&score_b) = b.get(term) {
            dot_product += score_a * score_b;
        }
    }

    for &score_b in b.values() {
        norm_b += score_b * score_b;
    }

    if norm_a == 0 || norm_b == 0 {
        return 0.0;
    }

    (dot_product as f64) / ((norm_a as f64).sqrt() * (norm_b as f64).sqrt())
}
