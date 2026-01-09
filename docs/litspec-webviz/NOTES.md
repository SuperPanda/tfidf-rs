# ============================================================================
# Pure morphism: Path → IndexName (no side effects)
# ============================================================================

get_index_name() {
    local path="$1"
    local rel=$(realpath "$path" --relative-to="$HOME" 2>/dev/null || echo "$path")
    local name=$(echo "$rel" | sed 's|^~/|home/|; s|^/||' | tr '/.' '_')
    echo "${TFIDF_CACHE_DIR}/${name}.index"
}
export -f get_index_name

# ============================================================================
# Morphism: Path → Index (side effect: creates file, outputs path)
# ============================================================================

build_index() {
    local path="$1"
    local index=$(get_index_name "$path")
    tfidf save "$index" "$path" >&2
    echo "$index"  # output for composition
}
export -f build_index

# ============================================================================
# Functor: List Paths → List Indices (parallel application)
# ============================================================================

PATHS_TO_INDEX="$HOME/notes/wiki:$HOME/repos/personal-wiki"

# The morphism is visible in the pipeline:
# String → [Path] → [Index]
echo "$PATHS_TO_INDEX" | 
    tr ':' '\n' |              # split: String → [Path]
    parallel build_index {}     # map: [Path] → [Index]


# «create index mappings» - Pure structure, lazy evaluation

#!/usr/bin/env bash
set -euo pipefail

TFIDF_CACHE_DIR="$HOME/.cache/tfidf-rs-dev"
mkdir -p "${TFIDF_CACHE_DIR}"

# ============================================================================
# Pure morphism: Path → (Path, IndexName)
# No side effects, just the mapping structure
# ============================================================================

get_index_mapping() {
    local path="$1"
    local rel=$(realpath "$path" --relative-to="$HOME" 2>/dev/null || echo "$path")
    local name=$(echo "$rel" | sed 's|^~/|home/|; s|^/||' | tr '/.' '_')
    local index="${TFIDF_CACHE_DIR}/${name}.index"
    echo -e "${path}\t${index}"  # The arrow: path ↦ index
}
export -f get_index_mapping

# ============================================================================
# Functor: [Path] → [(Path, Index)] - Pure structure
# ============================================================================

PATHS_TO_INDEX="$HOME/notes/wiki:$HOME/repos/personal-wiki:$HOME/docs"

# Generate the mapping (pure, no side effects)
MAPPINGS=$(echo "$PATHS_TO_INDEX" | 
    tr ':' '\n' | 
    parallel get_index_mapping {})

# Now we have the structure - we can reason about it before forcing evaluation
echo "$MAPPINGS" > index_mappings.tsv

# ============================================================================
# Effectful morphism: (Path, IndexName) → Index (builds file)
# Only execute when needed
# ============================================================================

realize_index() {
    local path="$1"
    local index="$2"
    
    if [[ ! -f "$index" ]]; then
        echo "Building: $path → $index" >&2
        tfidf save "$index" "$path"
    else
        echo "Cached: $index" >&2
    fi
    echo -e "${path}\t${index}"
}
export -f realize_index

# Force evaluation (optional):
cat index_mappings.tsv | parallel --colsep '\t' realize_index {1} {2}




# «index algebra» - Operations on [(Path, Index)] structures

# Union: Combine multiple mapping files
merge_mappings() {
    cat "$@" | sort -u -k1,1
}

# Intersection: Find common paths
intersect_mappings() {
    local f1="$1"
    local f2="$2"
    join -t $'\t' <(sort "$f1") <(sort "$f2")
}

# Difference: Find paths in first but not second
diff_mappings() {
    local f1="$1"
    local f2="$2"
    comm -23 <(cut -f1 "$f1" | sort) <(cut -f1 "$f2" | sort) |
        grep -F -f - "$f1"
}

# Map over indices: [(Path, Index)] → [(Path, Index')]
remap_indices() {
    local suffix="$1"
    cut -f1 index_mappings.tsv |
        parallel "echo -e '{}\t$(get_index_name {})${suffix}'"
}


# «cross-corpus edges» - Edges between index spaces

# Generate edges within each index space
generate_edges_by_corpus() {
    while IFS=$'\t' read -r corpus_path index group; do
        tfidf query --index "$index" list |
        parallel --jobs 4 "
            tfidf query --index $index similar {} --top 10 2>/dev/null |
            sed \"s|^|{}\t|; s|$|\t${index}\t${group}|\"
        "
    done
}

cat labeled_mappings.tsv | generate_edges_by_corpus > edges_with_corpus_id.tsv

# Find bridge edges (documents appearing in multiple indices)
find_bridges() {
    cut -f1 nodes_with_provenance.tsv | 
        sort | 
        uniq -c | 
        sed 's/^ *//' |
        awk '$1 > 1 {print $2}' |
        grep -F -f - nodes_with_provenance.tsv
}

# This shows which documents exist in multiple corpus spaces
find_bridges > bridge_nodes.tsv
