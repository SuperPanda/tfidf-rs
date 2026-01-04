# The TFIDF VIZ Experi<ence|ment>

<!--toc:start-->
- [The TFIDF VIZ Experi<ence|ment>](#the-tfidf-viz-experiencement)
  - [The `viz.html` File](#the-vizhtml-file)
    - [Requirements](#requirements)
      - [Requirement: Verify HTML rendering does not break with default parameters (no generated data)](#requirement-verify-html-rendering-does-not-break-with-default-parameters-no-generated-data)
      - [Requirement: Detect Drift from Executable Source](#requirement-detect-drift-from-executable-source)
    - [Generate A Reference Literate Design](#generate-a-reference-literate-design)
    - [Literate Setup](#literate-setup)
    - [The Graph Generator (File: `./generate-graph.sh`)](#the-graph-generator-file-generate-graphsh)
      - [Preamble](#preamble)
      - [Configuration](#configuration)
      - [Functions](#functions)
      - [Temporary Files](#temporary-files)
      - [Generate JSON from TSV (from the `tfidf` index)](#generate-json-from-tsv-from-the-tfidf-index)
      - [Summary / Sample](#summary-sample)
    - [Visualise document similarity with TFIDF](#visualise-document-similarity-with-tfidf)
    - [WIP: Later](#wip-later)
  - [Appendix](#appendix)
    - [`tfidf` Usage](#tfidf-usage)
  - [Manual `webdoc-pipe(1)`](#manual-webdoc-pipe1)
<!--toc:end-->


> **Relevant** [Shell Style Guide](~/notes/wiki/style-guides/shell-style-guide.md)

> **Note** The headers should be structured so that table of contents,
  are emergently beautiful (reword).

## The `viz.html` File

The `viz.html` file is generated on demand.

The following is structured as follows

| litblock | description |
| «viz.html.1» | Html before generated data |
| «viz.html.3» | Html after generated data |

### Requirements

Design Principles: Should support the initial morphism (the *base* case),
it should not need initial objects.

#### Requirement: Verify HTML rendering does not break with default parameters (no generated data)

> **Emergent Requirement 1: Verify HTML is Valid**
> 
> This should run with no generated data

**Double Check what the newest version of SWEBOK refers to this as...**

```sh «Emergent Requirement»
#!/usr/bin/env bash
export LITBLOCK_SRC="README.md" # THIS FILE
cat <(litblock "viz.html.1") <(litblock "viz.html.3") | webdoc-pipe
```

#### Requirement: Detect Drift from Executable Source

> **TODO:** Split into sections later.


## Verification


### Check for Drift

```sh «Check viz.html»
#!/usr/bin/env bash
export LITBLOCK_SRC="README.md" # THIS FILE
diff viz.html.1 <(litblock "viz.html.1")
diff viz.html.3 <(litblock "viz.html.3")
```

### Check reference `html.2` format

This can be used to represent a desired format the pipelines should be compatible with. 
Just add a literate block that matches some fixture.

```sh «Check viz.html»
cat <<'SAMPLE_GRAPH' | cat viz.html.1 - viz.html.3 | webdoc-pipe
    nodes: [
        { id: 'path/to/foo', name: 'foo', group: 1 },
        { id: 'path/to/bar', name: 'bar', group: 1 },
        { id: 'path/to/a/baz', name: 'baz', group: 2 }
    ],
    links: [
        { source: 'path/to/foo', target: 'path/to/bar', value: 0.4 },
        { source: 'path/to/bar', target: 'path/to/a/baz', value: 0.1 },
        { source: 'path/to/a/baz', target: 'path/to/bar', value: 0.9 }      
    ]
SAMPLE_GRAPH
```

## Setup


```sh «Create viz.html»
#!/usr/bin/env sh
export LITBLOCK_SRC="README.md" # THIS FILE
litblock "viz.html.1" > viz.html.1 # top
litblock "viz.html.3" > viz.html.3 # bottom
```
 
Later, I will use IDs and add file and node type (document or term). Link will be
like similarity, references, and other types of relations, between different categories.

> **Note** Upon execute the following, it will show the `changes` made

    cat README.md | litblock "Check viz.html" README.md | sh -
    webdoc-pipe open

> **Note** Once `webdoc-pipe` is open, just refresh the page,
>          to see changes. This is an intentional constraint of `webdoc-pipe`.


It should be noted that the graph generator `«graph-generator»` currently outputs a single `jsonl` object.


```html «viz.html.1»
<!doctype html>
<html lang="en">
  <head>
  <style>
  body { margin: 0; background-color: white; width: 100vw; height: 100vh;}
  #controls {
      position: absolute;
      display: none;
      top: 10px;
      left: 10px;
      background: rgba(0, 0, 0, 0.8);
      padding: 15px;
      border-radius: 5px;
      color: #0f0;
      z-index: 100;
      background: #00000000;
      max-width: 300px;

    }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/3d-force-graph"></script>
    <!--Commented below cause they don't work-->
  </head>
  <body>
<div id="controls">
  <label>strength: <input id="bloomStrength" type="range" min="0" max="30" step="0.01" value="1"/></label><br/>
  <label>radius:   <input id="bloomRadius"   type="range" min="0" max="20" step="0.01" value="0.3"/></label><br />
  <label>threshold:<input id="bloomThresh"    type="range" min="0" max="10" step="0.01" value="0.1"/></label><br />
</div>
    <div id="3d-graph"></div>
<script type="module">
  let highlightNodes = new Set();
  let highlightLinks = new Set();
  const data = {
```

This would contain the generated output.


```sh «viz.html.3»
}
  import * as THREE from 'https://esm.sh/three';
   import { UnrealBloomPass } from 'https://esm.sh/three/examples/jsm/postprocessing/UnrealBloomPass.js';
  const myGraph = new ForceGraph3D(document.getElementById('3d-graph'), {})
    .nodeThreeObjectExtend(true)
  // const myGraph = ForceGraph3D(document.getElementById('3d-graph'), {})
  .nodeThreeObject(node => {
    const label = node.name;

    const canvas = document.createElement('canvas');
    const size = 512;
    
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
    const material = new THREE.SpriteMaterial({ map: texture, transparent: true, depthWrite: false });
    const sprite = new THREE.Sprite(material);

    // Adjust scale to control text size in the scene
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
        // Aim at node from outside it
        const distance = 40;
        const distRatio = 1 + distance/Math.hypot(node.x, node.y, node.z);

        const newPos = node.x || node.y || node.z
          ? { x: node.x * distRatio, y: node.y * distRatio, z: node.z * distRatio }
          : { x: 0, y: 0, z: distance }; // special case if node is in (0,0,0)

        myGraph.cameraPosition(
          newPos, // new position
          node, // lookAt ({ x, y, z })
          3000  // ms transition duration
        );
     })
     .nodeAutoColorBy('group')
     .graphData(data);
     const bloomPass = new UnrealBloomPass();
     bloomPass.params =   new THREE.Vector2(window.innerWidth * 3.0, window.innerHeight * 4.0);
     bloomPass.strength = 1;
     bloomPass.radius = 0.3;
     bloomPass.threshold = 0.3;
    myGraph.postProcessingComposer().addPass(bloomPass);
    // pull out the underlying three.js renderer, camera & controls:
    const renderer = myGraph.renderer();         // WebGLRenderer
    const camera   = myGraph.camera();           // PerspectiveCamera
    const controls = myGraph.controls();         // OrbitControls

    function onWindowResize() {
      const width  = window.innerWidth;
      const height = window.innerHeight;

      // 1) resize the three.js renderer
      renderer.setSize(width, height);

      // 2) update the camera
      camera.aspect = width / height;
      camera.updateProjectionMatrix();

      // 3) notify orbit-controls so it can update its internal state
      //    (so mouse pans/orbits still map 1:1 to the new viewport size)
      controls.handleResize();
    }

// listen for window resizes
window.addEventListener('resize', onWindowResize);

// fire it once to get the initial size correct
onWindowResize();
    </script>

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
</script>
  </body>
</html>
```

### The Graph Generator (File: `./generate-graph.sh`)

#### Preamble

```bash «generate-graph.sh»
#!/usr/bin/env bash
# generate-graph.sh - Generate k-NN similarity graph from TF-IDF index
#
# Usage:
#   ./generate-graph.sh <index-file> [top-n] [output.json]
#
# Outputs JSON format:
#   {
#     "nodes": [{"id": "path", "name": "basename", "group": 0}, ...],
#     "links": [{"source": "path1", "target": "path2", "value": 0.85}, ...]
#   }
#
# Example:
#   ./generate-graph.sh docs.index 10 graph.json
#   ./generate-graph.sh docs.index 5 | jq '.nodes | length'

set -euo pipefail
```

#### Configuration

```bash «generate-graph.sh»
# Configuration
INDEX="${1:?Usage: $0 <index-file> [top-n] [output.json]}"
TOP_N="${2:-10}"
OUTPUT="${3:-/dev/stdout}"

# Check if index exists
if [[ ! -f "$INDEX" ]]; then
  echo "Error: Index file not found: $INDEX" >&2
  exit 1
fi

# Check if tfidf is available
if ! command -v /usr/local/bin/tfidf &> /dev/null; then
  echo "Error: tfidf.rs not found in current directory" >&2
  exit 1
fi
```

#### Functions

```bash «generate-graph.sh»
# Progress indicator to stderr if output is not stdout
show_progress() {
  if [[ "$OUTPUT" != "/dev/stdout" ]]; then
    echo "$@" >&2
  fi
}

show_progress "Generating similarity graph..."
```

#### Temporary Files

Temporary files are used to store the directed edges `RAW_EDGES_TSV` (e.g. top similar files, etc.),
the undirected edges after duplication, `EDGES_TSV` and the list ofNODES_LIST

```bash «generate-graph.sh»
# Temporary files
RAW_EDGES_TSV=$(mktemp)     # directed edges: src \t dest \t weight
EDGES_TSV=$(mktemp)         # undirected, deduped: a \t b \t weight
NODES_LIST=$(mktemp)        # unique node IDs
trap 'rm -f "$RAW_EDGES_TSV" "$EDGES_TSV" "$NODES_LIST"' EXIT
```

#### Generate JSON from TSV (from the `tfidf` index)

> **Need:** Functions need to be reusable codeblocks to allow the type of data to be 
>   generalised.
>          
> There need to be clearer boundaries so the different stages can be reused.
>
> This was to move away from `AWK` to make this easier. (Architecturally, `AWK`
> is forbidden, and loops ideally are converted to maps and reduce, using `parallel`.
>
> Functions are easier to compose, if functions are exported it can be used by `parallel`.


```bash «generate-graph.sh»
########################################
# 1. Generate directed edges
########################################

# /usr/local/bin/tfidf query --index "$INDEX" list \
#   prints one document path per line.
#
# For each source_doc, we run 'similar' and prepend the source id.
#
# This replaces:
#   awk -v src="$source_doc" '{print src "\t" $1 "\t" $2}'
#
/usr/local/bin/tfidf query --index "$INDEX" list | while IFS= read -r source_doc; do
  /usr/local/bin/tfidf query --index "$INDEX" similar "$source_doc" --top "$TOP_N" 2>/dev/null \
    | sed "s~^~${source_doc}\t~" || true

  if [[ "$OUTPUT" != "/dev/stdout" ]]; then
    printf "." >&2
  fi
done > "$RAW_EDGES_TSV"

if [[ "$OUTPUT" != "/dev/stdout" ]]; then
  echo "" >&2
fi

########################################
# 2. Symmetrize and deduplicate edges
########################################
#
# Goal: for each pair (u,v), keep only one (min(u,v), max(u,v)) with weight
# corresponding to one of the occurrences (if there are multiple).
#
# Steps:
#  - sort so that fields 1 and 2 are alphabetical
#  - rewrite each line as: smaller\tlarger\tweight
#  - sort and unique them by first 2 fields, keeping first weight seen

# Normalize each edge so smaller node is first.
# We let 'sort' order the first two fields, then re-write them stably.
sort -u "$RAW_EDGES_TSV" | while IFS=$'\t' read -r a b w; do
  # compare strings lexicographically
  if [[ "$a" < "$b" ]]; then
    printf '%s\t%s\t%s\n' "$a" "$b" "$w"
  else
    printf '%s\t%s\t%s\n' "$b" "$a" "$w"
  fi
done | sort -u -t$'\t' -k1,1 -k2,2 > "$EDGES_TSV"

########################################
# 3. Build node list
########################################
#
# Extract first and second columns, unify, sort unique.

cut -f1 "$EDGES_TSV" > "$NODES_LIST"
cut -f2 "$EDGES_TSV" >> "$NODES_LIST"
sort -u "$NODES_LIST" -o "$NODES_LIST"

########################################
# 4. Emit JSON
########################################

{
  echo "{"
  echo '  "nodes": ['

  node_count=0
  total_nodes=$(wc -l < "$NODES_LIST" | tr -d ' ')

  # Emit nodes
  while IFS= read -r path; do
    node_count=$((node_count + 1))

    # basename: strip everything up to last /
    basename="${path##*/}"
    group=0

    printf '    {"id": "%s", "name": "%s", "group": %d}' \
      "$(printf '%s' "$path"    | sed 's/"/\\"/g')" \
      "$(printf '%s' "$(basename $path .md)" | sed 's/"/\\"/g')" \
      "$group"

    if [ "$node_count" -lt "$total_nodes" ]; then
      printf ','
    fi
    printf '\n'
  done < "$NODES_LIST"

  echo "  ],"
  echo '  "links": ['

  link_count=0
  total_links=$(wc -l < "$EDGES_TSV" | tr -d ' ')

  # Emit links
  while IFS=$'\t' read -r source target value; do
    link_count=$((link_count + 1))
    printf '    {"source": "%s", "target": "%s", "value": %s}' \
      "$(printf '%s' "$source" | sed 's/"/\\"/g')" \
      "$(printf '%s' "$target" | sed 's/"/\\"/g')" \
      "$value"

    if [ "$link_count" -lt "$total_links" ]; then
      printf ','
    fi
    printf '\n'
  done < "$EDGES_TSV"

  echo "  ]"
  echo "}"
} > "$OUTPUT"
```

#### Summary / Sample

```bash «generate-graph.sh»
########################################
# 5. Summary / sample (if writing to file)
########################################

if [[ "$OUTPUT" != "/dev/stdout" ]]; then
  if command -v jq >/dev/null 2>&1; then
    NODES=$(jq '.nodes | length' "$OUTPUT" 2>/dev/null || echo "?")
    EDGES=$(jq '.links | length' "$OUTPUT" 2>/dev/null || echo "?")
  else
    # Fallback without jq
    NODES=$(grep -c '"id":' "$OUTPUT" 2>/dev/null || echo "?")
    EDGES=$(grep -c '"source":' "$OUTPUT" 2>/dev/null || echo "?")
  fi

  show_progress "Generated graph: $NODES nodes, $EDGES links → $OUTPUT"
  show_progress ""
  show_progress "Sample links (first 3):"
  if command -v jq >/dev/null 2>&1; then
    jq -r '
      .links[:3] | .[] |
      "  \(.value | tonumber * 1000 | floor / 1000)  \(.source)\n         ← → \(.target)\n"
    ' "$OUTPUT" 2>/dev/null | head -9 >&2 || true
  else
    # Simple textual sample without jq
    sed -n '1,20p' "$EDGES_TSV" >&2 || true
  fi
fi
```

> **Pain point** This should be converted to rust, as it is pretty time consuming.


### Visualise document similarity with TFIDF

Let's save the index of `~/notes/wiki` using: `tfidf save ~/.cache/wiki.tfidf.index ~/notes/wiki`

And visualise it.

First I'm going to review my `mcfly dump --since yesterday | jq .[] -c | grep tfidf | grep wiki` output.

Plan: Adding three sources of data I want to merge as separate indices
or organise, i will look at them individually, as ii've
done this before and i will show you something cool.

Hmmm.. I think tfidf is running the rust-script version, so
it is not showing up in the process manager


```bash «create indices»
#!/usr/bin/env bash
TFIDF_CACHE_DIR="$HOME/.cache/tfidf-rs-dev"
mkdir  -p "${TFIDF_CACHE_DIR}"
# note this is for personal use, don't use this in a production
# system, I'm going to find errors the hard way to move fast.
# but I recommend solving this properly.
get_index_name() {
    path="${1:-~}"
    index_name="${2:-""}"
    path="$(realpath "$path" --relative-to="$HOME")"
    if [ "$index_name" = "" ]; then
        index_name="$(echo "${path}" | sed 's/^~/home/' | tr '/.' '._').index"
    fi
    echo "${TFIDF_CACHE_DIR}/${index_name}"
}

export -f get_index_name

build_index() {
    local path="$1"
    local index=$(get_index_name "$path")
    tfidf save "$index" "$path" >&2
    echo "$index"  # output for composition
}
export -f build_index


PATHS_TO_INDEX="~/notes/wiki:~/repos/personal-wiki"
export TFIDF_INDEX="$(get_index_name ~/notes/wiki)"
echo "$PATHS_TO_INDEX" | xargs -d: echo
echo $PATHS_TO_INDEX | tr ':' '\n' | parallel -d\n -0 tfidf save "${TFIDF_INDEX}"  "{}"
```



```

```


export FD_OPTS="-t f -e md" 
**Use fd.. to combine the output of `at <(echo "${PATHS_TO_INDEX}" | xargs -d: tfidf query files ) | parallel cat ${tfidf query files {} | parallel -0 fd $FD_OPTS . {}**
export FD_OPTS="-t f -e md" 
export FD_OPTS="-t f -e d2" 
things that should be removed, as identified etc.
maybe need to printf... but we want to get a pattern going... so 
we can use the set of files from different paths using sort and uniq
using the set of paths of paths to the directory  for each thing
to be able to assign groups...
... but unlike above, that should be composable and not embedded in logic.
do this bit first...

tfidf save "${TFIDF_CACHE}" ~/notes/wiki
tfidf query --index "${TFIDF_CACHE}"

    Put image of the included image files region.

    Put the d2 loose file images here.

You will see that most of this discovery process is
done without looking at the content (Yoneda anyone)

But we will also want to look at the terms as well but thats
later. Maybe we can do the term weighting per document
or something. I swear I had that capability in an unsaved version.

    Put image of the included image files region.

    Put the d2 loose file images here.


"./generate-graph.sh ~/.local/share/tfidf-rs-dev/wiki.index 5 index.html.2.raw"



### WIP: Later

**This section contains commented out sketches of whats coming...**

<!--
### Next... topologically slicing the discovered items discovered.




### Generate 


### Future Work

Generating a makefile, integrating the literate `devop book` using the `architecture book` ,
to improve the `ARCHITECTURE.md` document.

Using `vlist` to execute simultaneous changes with `terminal-ai` at the same time, and have
it discover the pain points in compositionality, and discover ways to refine the interfaces,
at a higher level.

-->



## Appendix

### `tfidf` Usage

Generated with:

    tfidf 2>&1

> **TODO** Fix the stupid appname examples, make it like tdidf-dev, or whatever,
           it should support multiple repos in the same prefix (in state or whatever),
           and the config should be in the namespace
           It's when you want to refresh your settings, or running a dev or test thing.
           Don't go around rename things like that randomly you will shoot yourself in the foot again.

```USAGE «tfidf help»
tfidf - composable document similarity

USAGE
  tfidf save <index> [PATH...] [--]
  tfidf query [--index <path>] <subcommand>

SAVE
  Build an index from files and directories.
  After --, read additional paths from stdin (one per line).

QUERY SUBCOMMANDS
  list                    List all indexed documents
  terms [--top N]         Show top N terms (default: 20)
  similar <file> [--top N]  Find similar documents (default: 10)

INDEX RESOLUTION
  1. --index flag
  2. TFIDF_INDEX environment variable
  3. ./tfidf.index

CONFIGURATION
  Stopwords: $XDG_CONFIG_HOME/$TFIDF_APPNAME/stopwords.txt
             (defaults to $XDG_CONFIG_HOME/tfidf/stopwords.txt)
  
  Set TFIDF_APPNAME to use separate configurations:
    export TFIDF_APPNAME=project-docs
    mkdir -p ~/.config/project-docs

OUTPUT
  Tab-delimited for composition with cut, sort, xargs, etc.

EXAMPLES
  # Save index
  tfidf save docs.index src/ notes/ --

  # Query
  tfidf query list | head -5
  tfidf query terms --top 100 | sort -k2,2nr
  tfidf query similar main.rs | cut -f1 | xargs cat
  
  # Use separate configuration
  TFIDF_APPNAME=work tfidf save work.index ~/work/docs
```

## Manual `webdoc-pipe(1)`

Generated with:

    man --local-file ~/repos/webdoc-pipe/man/man.1 

```
WEBDOC-PIPE(1)                   User Commands                   WEBDOC-PIPE(1)

NAME
       webdoc-pipe - Serve piped HTML/CSS/JS as three HTTP files

SYNOPSIS
       webdoc-pipe open
       webdoc-pipe url
       webdoc-pipe stop
       webdoc-pipe html
       webdoc-pipe css
       webdoc-pipe js

DESCRIPTION
       webdoc-pipe  maintains  an in-memory web document and serves it via HTTP
       at:

              http://127.0.0.1:PORT/

       It serves exactly three files:

              • / or /index.html → HTML content

              • /style.css → CSS content

              • /script.js → JavaScript content

       Content is updated by piping to subcommands:

              cat index.html | webdoc-pipe html

       The daemon starts automatically and runs per-user.

COMMANDS
       open   Open the document in Firefox (or xdg-open).

       url    Print the document URL.

       stop   Stop the daemon.

       html   Read stdin as HTML (served as /index.html).

       css    Read stdin as CSS (served as /style.css).

       js     Read stdin as JavaScript (served as /script.js).

FILES
       $XDG_RUNTIME_DIR/webdoc-pipe-*.sock
              Unix domain socket.

       $XDG_RUNTIME_DIR/webdoc-pipe.current
              Daemon metadata (PID, port, socket path).

ENVIRONMENT
       WEBDOC_BROWSER
              Browser command (default: firefox).

SEE ALSO
       firefox(1), xdg-open(1)

AUTHOR
       Andrew Briscoe

LICENSE
       WTFPL

v0.1.0                             2025-10-01                    WEBDOC-PIPE(1)
```


