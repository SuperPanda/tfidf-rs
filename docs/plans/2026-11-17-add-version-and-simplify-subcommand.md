# Proposed change: version commands and positional-index removal

For the attached `tfidf-rs` crate, acceptance test is that the version
command is returned.

```console
$ tfidf -v
tfidf 0.1.0
$ tfidf --version
tfidf 0.1.0
$ tfidf version
tfidf 0.1.0
```

```sh «acceptance test»
LOCAL_VERSION="$(tfidf -v)" SRC_VERSION="$(grep -P '^version = ' Cargo.toml | cut -f2- -d=)" # | xargs semver -i minor
printf  "Install Version: %s\n" "$LOCAL_VERSION"
printf  "Build Version: %s\n" "$(target/release/tfidf -v)"
printf "Dev Version: %s\n" "$SRC_VERSION"

echo "Pass when PATH is installed via pacman."
```

Outputs:

LOCAL_VERSION="$(tfidf -v)" SRC_VERSION="$(grep -P '^version = ' Cargo.toml | cut -f2- -d=)" # | xargs semver -i minor
printf "Install Version: %s\n" "$LOCAL_VERSION"
printf  "Build Version: %s\n" "$(target/release/tfidf -v)"
printf "Dev Version: %s\n" "$SRC_VERSION"
echo "tfidf --version"

Contract:

- Source the version exclusively from `env!("CARGO_PKG_VERSION")`.
- Use the executable’s public name, `tfidf`, in the output; the package remains `tfidf-rs`.
- Write one newline-terminated line to stdout, exit successfully, and leave stderr empty.
- Handle these commands before configuration loading, index resolution, or filesystem access.
- Use the existing broken-pipe-safe output mechanism.
- Document all three forms in help.

I would initially support these as **top-level commands**, not interpret `-v` inside search arguments or filenames.

### Remove the legacy positional index destination

I interpret “remove the deprecated positional `--index` file” as removing:

```sh
tfidf save docs.index src/
```

as an alternative way to specify the **destination**, while retaining:

```sh
tfidf save --index docs.index src/
```

Specifically:

- Delete `should_treat_first_save_arg_as_legacy_index`.
- Treat every positional save argument as an input path, regardless of its name, extension, or existence.
- Always resolve the destination using:

  ```text
  --index > TFIDF_INDEX > resolved data directory/tfidf.index
  ```

- Remove the legacy-form documentation and example.

An existing input named `docs.index` must remain valid. Removing the positional destination should not introduce a blanket rejection of `.index` inputs.

## 2. Tests before implementation

Add native CLI integration tests that execute the built `tfidf` binary. Give every test its own temporary working directory and isolated environment; avoid mutating the test process’s environment.

| Acceptance test                                                                                                        | Expected before change |
| ---------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| Each version spelling produces exactly `tfidf <Cargo version>\n`, with successful status and empty stderr              | Fails                  |
| Version commands succeed with deliberately unusable configuration/index paths                                          | Fails                  |
| `save existing.index other.txt` indexes both inputs into the resolved destination without overwriting `existing.index` | Fails                  |
| `save missing.index other.txt` does not reinterpret the missing input as the destination                               | Fails                  |

Add regression coverage for the touched behaviour too:

- Explicit `--index` still selects the destination and is not indexed as input.
- Destination precedence and each existing data-directory fallback remain intact.
- Ordinary positional inputs and stdin path mixing still work.
- Help advertises version commands and no longer advertises the legacy form.
- Missing `--index` values still fail clearly.

The missing-input test should preserve the current discovery behaviour—silently skipping nonexistent inputs—rather than quietly adding another behavioural change.

**I cannot execute tests against the attachments here**, so these are proposed red/green checks, not claimed test results. Record the actual failing run before applying production changes.

## 3. Follow-up: transcript resources and views

- **WASI:** exposes capability-scoped guest directories such as `/transcripts` and `/notes`.
- **Resource identity:** identifies a transcript, message, or chunk independently of its filesystem location.
- **TF-IDF:** ranks those resource IDs; it should not require “one transcript = one file.”
- **Views:** select resources and relationships, allowing the same resource to appear in overlapping tagged views without duplicating its identity.
- **MCP adapter:** exposes resource listing/reading and a search tool returning the five highest-ranked resources.
- **Varlink/config schema:** a separate interface decision, not something filesystem mounts or MCP automatically provide.

Two implementation cautions:

1. The Rust target is likely **`wasm32-wasip1`**, depending on your Wasmtime setup. WASI directory access is not itself MCP support.
2. The existing Rayon-backed code needs an actual Wasmtime indexing/search smoke test; compiling for WASI alone does not establish that its parallel execution paths work.

For the first Lua demo, I’d use **search → five resource IDs → read selected resources**, leaving neighbour expansion and cached views for the next increment.

**Approval scope:** version aliases, removal of positional destination inference, documentation, and corresponding tests—without changing index format, ranking, dependencies, or adding an MCP server yet.
