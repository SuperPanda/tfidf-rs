# `tfidf-rs` → k‑NN graph → 3D viz (terminal‑friendly, litblock‑ready)

This document is meant to be used with `litblock`:

```sh
litblock --list tfidf-similarity-viz.md
litblock "generate-graph" tfidf-similarity-viz.md > generate-graph.rs
```

We build:

- a Rust `generate-graph` binary (fast k‑NN graph from your TF‑IDF index)
- HTML with wired bloom controls
- a terminal pipeline that composes with `jq`, `rg`, `webdoc-pipe`, etc.

---

## Blocks

### 1. `generate-graph.rs` (Rust k‑NN graph generator) «generate-graph»

```rust «generate-graph»
#!/usr/bin/env -S rust-script
//! ```cargo
//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! rayon = "1.8"
//! ```
//!
//! generate-graph.rs
//!
//! Build an undirected k-NN similarity graph from a tfidf-rs binary index.
//!
//! Usage:
//!   generate-graph <index-file> [--top N] [--out FILE]
//!
//! JSON output (to stdout or --out FILE):
//!   {
//!     "nodes": [{ "id": "path", "name": "basename", "group": 0 }, ...],
//!     "links": [{ "source": "path1", "target": "path2", "value": 0.85 }, ...]
//!   }
//!
//! Compositional CLI examples:
//!   # Just nodes as lines
//!   generate-graph docs.index | jq -r '.nodes[].id'
//!
//!   # Edge list for rg / graph toy
//!   generate-graph docs.index | jq -r '.links[] | "\(.source)\t\(.target)\t\(.value)"'
//!
//!   # Filter to a neighborhood around paths matching "devops"
//!   generate-graph docs.index \
//!     | jq --arg re "devops" '
//!         . as $g
//!         | $g.nodes
//!         | map(select(.id | test($re))) as $hits
//!         | $g
//!         | .links |= map(select(
//!             any($hits[].id; .source == . or .target == .)
//!           ))
//!       '
//!
//! It talks directly to the *binary* tfidf index you already have; it does
//! **not** shell out to the `tfidf` CLI, so it’s fast and parallel.

use rayon::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Copied minimal structures from tfidf-rs so we can deserialize the same index.
/// Keep this in sync with your tfidf-rs `Index` / `Document` definitions.
/// (Here we only need path + TF-IDF vector, but we deserialize the whole thing.)

type FastMap<K, V> = HashMap<K, V>;

#[derive(Clone, Deserialize)]
struct Document {
    id: u32,
    path: String,
    terms: FastMap<String, u32>,
    length: u32,
}

#[derive(Deserialize)]
struct Index {
    docs: Vec<Document>,
    term_df: FastMap<String, u32>,
}

impl Index {
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
}

fn cosine_similarity(a: &FastMap<String, f64>, b: &FastMap<String, f64>) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (term, &sa) in a {
        norm_a += sa * sa;
        if let Some(&sb) = b.get(term) {
            dot += sa * sb;
        }
    }
    for &sb in b.values() {
        norm_b += sb * sb;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

#[derive(Serialize)]
struct JsonNode {
    id: String,
    name: String,
    group: u32,
}

#[derive(Serialize)]
struct JsonLink {
    source: String,
    target: String,
    value: f64,
}

#[derive(Serialize)]
struct JsonGraph {
    nodes: Vec<JsonNode>,
    links: Vec<JsonLink>,
}

fn usage() -> ! {
    eprintln!(
        "Usage:
  generate-graph <index-file> [--top N] [--out FILE]

Options:
  --top N      Top N similar neighbors per node (default: 10)
  --out FILE   Write JSON graph to FILE instead of stdout

Examples:
  generate-graph docs.index | jq '.nodes | length'
  generate-graph docs.index --top 3 --out graph.json"
    );
    std::process::exit(1);
}

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage();
    }

    let index_path = args.remove(0);
    let mut top_n: usize = 10;
    let mut out_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--top" if i + 1 < args.len() => {
                top_n = args[i + 1].parse().unwrap_or(10);
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out_path = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                usage();
            }
        }
    }

    // Load tfidf index via bincode
    let file = File::open(&index_path)
        .unwrap_or_else(|e| {
            eprintln!("Error opening index {}: {}", index_path, e);
            std::process::exit(1);
        });
    let index: Index = bincode::deserialize_from(file).unwrap_or_else(|e| {
        eprintln!("Error deserializing index (bincode): {}", e);
        std::process::exit(1);
    });

    // Precompute TF-IDF vectors for all docs (parallel)
    let vectors: Vec<FastMap<String, f64>> = index
        .docs
        .par_iter()
        .map(|d| index.vector(d))
        .collect();

    // For each document, find top-N most similar others (directed edges)
    let directed_edges: Vec<(usize, usize, f64)> = (0..index.docs.len())
        .into_par_iter()
        .flat_map(|i| {
            let v_i = &vectors[i];
            let mut sims: Vec<(usize, f64)> = (0..index.docs.len())
                .filter(|&j| j != i)
                .map(|j| {
                    let s = cosine_similarity(v_i, &vectors[j]);
                    (j, s)
                })
                .filter(|&(_, s)| s > 0.0)
                .collect();

            sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            sims.truncate(top_n);

            sims.into_iter().map(move |(j, s)| (i, j, s))
        })
        .collect();

    // Symmetrize + dedup: for each unordered pair (a,b), store one edge
    let mut undirected: HashMap<(usize, usize), f64> = HashMap::new();
    for (i, j, s) in directed_edges {
        let (a, b) = if i < j { (i, j) } else { (j, i) };
        // if both directions exist we could choose max or avg; take max for now
        undirected
            .entry((a, b))
            .and_modify(|existing| {
                if s > *existing {
                    *existing = s;
                }
            })
            .or_insert(s);
    }

    // Build node list (all docs that appear in at least one edge)
    let mut node_ids: HashSet<usize> = HashSet::new();
    for &(a, b) in undirected.keys() {
        node_ids.insert(a);
        node_ids.insert(b);
    }

    // Make nodes
    let mut nodes = Vec::with_capacity(node_ids.len());
    for idx in 0..index.docs.len() {
        if node_ids.contains(&idx) {
            let doc = &index.docs[idx];
            let name = Path::new(&doc.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&doc.path)
                .to_string();

            // you can turn group into a topology-based bucket later
            let group = 0u32;

            nodes.push(JsonNode {
                id: doc.path.clone(),
                name,
                group,
            });
        }
    }

    // Make links
    let mut links = Vec::with_capacity(undirected.len());
    for ((a, b), s) in undirected {
        let src = &index.docs[a].path;
        let dst = &index.docs[b].path;
        links.push(JsonLink {
            source: src.clone(),
            target: dst.clone(),
            value: s,
        });
    }

    // Stable sort for deterministic output (nice for diffs / jq)
    links.sort_by(|x, y| {
        x.source
            .cmp(&y.source)
            .then_with(|| x.target.cmp(&y.target))
    });

    let graph = JsonGraph { nodes, links };

    match out_path {
        Some(p) => {
            let f = File::create(&p)?;
            let mut w = BufWriter::new(f);
            serde_json::to_writer_pretty(&mut w, &graph).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("json encode error: {}", e))
            })?;
            w.flush()?;
        }
        None => {
            let stdout = io::stdout();
            let mut w = BufWriter::new(stdout.lock());
            serde_json::to_writer_pretty(&mut w, &graph).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("json encode error: {}", e))
            })?;
            w.write_all(b"\n")?;
            w.flush()?;
        }
    }

    Ok(())
}
```

Usage from shell (no Bash glue, fully composable):

```sh
# build index (you already do this)
fd --follow -t f -e md . ~/repos/books/the-{devops,note}-book/ \
  | tfidf save docs.index --

# fast graph build
generate-graph docs.index --top 3 --out graph.json

# inspect with jq
jq '.nodes | length' graph.json
jq '.links[:5]' graph.json

# produce middle HTML chunk from graph.json
jq '.
  | "  \"nodes\": " + ( .nodes | tojson ) + ",\n  \"links\": " + ( .links | tojson )
' -r graph.json > index.html.2
```

---

### 2. HTML header with bloom UI wired up «index.html.1»

You already almost have this; here it’s cleaned up to match the bloom wiring in your `index.html.3`.

```html «index.html.1»
<!doctype html>
<html lang="en">
  <head>
    <style>
      body {
        margin: 0;
        background-color: white;
        width: 100vw;
        height: 100vh;
      }
      #controls {
        position: absolute;
        top: 10px;
        left: 10px;
        padding: 15px;
        border-radius: 5px;
        color: #0f0;
        z-index: 100;
        background: #00000000;
        max-width: 300px;
      }
      #controls label {
        font-family: system-ui, sans-serif;
        font-size: 12px;
        display: flex;
        justify-content: space-between;
      }
      #controls input[type=range] {
        width: 160px;
      }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/3d-force-graph"></script>
  </head>
  <body>
    <div id="controls">
      <label>
        strength:
        <input id="bloomStrength" type="range"
               min="0" max="30" step="0.01" value="1" />
      </label>
      <label>
        radius:
        <input id="bloomRadius" type="range"
               min="0" max="20" step="0.01" value="0.3" />
      </label>
      <label>
        threshold:
        <input id="bloomThresh" type="range"
               min="0" max="10" step="0.01" value="0.1" />
      </label>
    </div>
    <div id="3d-graph"></div>
    <script type="module">
      let highlightNodes = new Set();
      let highlightLinks = new Set();
      const data = {
```

You can still adjust ranges/steps; the important bit is that the element IDs match what we use in the script.

---

### 3. HTML tail: labels + bloom wiring + resize + controls «index.html.3»

This is basically your current `index.html.3` with:

- bloom controls hooked directly into `bloomPass`
- minor cleanup on `UnrealBloomPass` init
- resize wired to `renderer`/`camera`/`controls`

```html «index.html.3»
}
  // Using 3d-force-graph UMD global (from the CDN script tag).
  import * as THREE from 'https://esm.sh/three';
  import { UnrealBloomPass }
    from 'https://esm.sh/three/examples/jsm/postprocessing/UnrealBloomPass.js';

  const myGraph = new ForceGraph3D(document.getElementById('3d-graph'), {})
    .nodeThreeObjectExtend(true)
    .nodeThreeObject(node => {
      const label = node.name;

      const canvas = document.createElement('canvas');
      const size = 256;
      canvas.width = size;
      canvas.height = size;

      const ctx = canvas.getContext('2d');
      ctx.clearRect(0, 0, size, size);
      ctx.fillStyle = 'white';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.font = '24px sans-serif';
      ctx.fillText(label, size / 2, size / 2);

      const texture = new THREE.CanvasTexture(canvas);
      texture.minFilter = THREE.LinearFilter;

      const material = new THREE.SpriteMaterial({
        map: texture,
        transparent: true,
        depthWrite: false
      });
      const sprite = new THREE.Sprite(material);

      const scale = 15;
      sprite.scale.set(scale, scale, 1);
      sprite.position.set(0, 7, 0);

      return sprite;
    })
    .linkDirectionalParticles(d => d.value * 10)
    .linkCurvature(d => d.value)
    .linkDirectionalParticleWidth(d => d.value)
    .nodeLabel('id')
    .linkOpacity(0.1)
    .onNodeClick(node => {
      const distance = 40;
      const distRatio = 1 + distance / Math.hypot(node.x, node.y, node.z);

      const newPos = (node.x || node.y || node.z)
        ? { x: node.x * distRatio,
            y: node.y * distRatio,
            z: node.z * distRatio }
        : { x: 0, y: 0, z: distance };

      myGraph.cameraPosition(
        newPos,
        node,
        3000
      );
    })
    .nodeAutoColorBy('group')
    .graphData(data);

  // Bloom
  const size = new THREE.Vector2(window.innerWidth, window.innerHeight);
  const bloomPass = new UnrealBloomPass(size, 1.0, 0.3, 0.3);

  myGraph.postProcessingComposer().addPass(bloomPass);

  // Pull out the underlying three.js pieces
  const renderer = myGraph.renderer();
  const camera   = myGraph.camera();
  const controls = myGraph.controls();

  function onWindowResize() {
    const width  = window.innerWidth;
    const height = window.innerHeight;

    renderer.setSize(width, height);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();

    // OrbitControls from 3d-force-graph exposes this:
    if (typeof controls.handleResize === 'function') {
      controls.handleResize();
    }
  }

  window.addEventListener('resize', onWindowResize);
  onWindowResize();
    </script>

    <!-- Bloom controls wired to bloomPass -->
    <script>
      const strengthSlider = document.getElementById('bloomStrength');
      const radiusSlider   = document.getElementById('bloomRadius');
      const threshSlider   = document.getElementById('bloomThresh');

      function updateBloom() {
        bloomPass.strength  = +strengthSlider.value;
        bloomPass.radius    = +radiusSlider.value;
        bloomPass.threshold = +threshSlider.value;
      }

      [strengthSlider, radiusSlider, threshSlider]
        .forEach(el => el.addEventListener('input', updateBloom));

      // Initialize with current slider values
      updateBloom();
    </script>
  </body>
</html>
```

Now the sliders directly control `bloomPass.*`, and everything resizes correctly with the window.

---

### 4. Steps: index → Rust graph → HTML → browser «STEPS.md»

```markdown «STEPS.md»
# Steps: TF‑IDF → Rust k‑NN graph → 3D viz

```bash
# 1. Build the TF-IDF index from markdown docs
fd --follow -t f -e md . ~/repos/books/the-{devops,note}-book/ \
  | tfidf save docs.index --

# 2. Build similarity graph with Rust (fast)
litblock generate-graph tfidf-similarity-viz.md > generate-graph.rs
chmod +x generate-graph.rs
./generate-graph.rs docs.index --top 3 --out graph.json

# 3. Convert graph.json into the middle HTML chunk
jq '.
  | "  \"nodes\": " + ( .nodes | tojson ) + ",\n  \"links\": " + ( .links | tojson )
' -r graph.json > index.html.2

# 4. Glue the three HTML parts together
litblock index.html.1 tfidf-similarity-viz.md > index.html.1
litblock index.html.3 tfidf-similarity-viz.md > index.html.3
cat index.html.{1,2,3} > index.html

# 5. Use webdoc-pipe for a terminal-friendly dev loop
cat index.html | webdoc-pipe html
webdoc-pipe open
```

You can re-run step 2–5 after tweaking `generate-graph.rs` or the HTML
partials and just keep the browser window open; `webdoc-pipe` will update
in place.
```

---

### 5. Setup helper «setup.sh»

```sh «setup.sh»
#!/usr/bin/env -S bash
set -euo pipefail

FILE=tfidf-similarity-viz.md

# Materialize blocks
litblock generate-graph "$FILE" > generate-graph.rs
litblock index.html.1 "$FILE"   > index.html.1
litblock index.html.3 "$FILE"   > index.html.3
litblock STEPS.md "$FILE"       > STEPS.md

chmod +x generate-graph.rs

echo "Generated: generate-graph.rs, index.html.{1,3}, STEPS.md"
echo "Now run:"
echo "  bash STEPS.md"
