#compdef tfidf

# zsh completion for tfidf
#
# Install:
#   mkdir -p ~/.zsh/completions
#   cp _tfidf ~/.zsh/completions/_tfidf
#   fpath=(~/.zsh/completions $fpath)
#   autoload -Uz compinit && compinit
#
# Dynamic completions:
#   - tfidf query similar <file> completes indexed document paths by calling:
#       tfidf query [--index <path>] list
#   - --index is detected from the current command line when present.
#   - Otherwise tfidf itself resolves TFIDF_INDEX / XDG defaults.

_tfidf_index_args_from_words() {
  local -a out
  local i

  out=()
  for (( i = 1; i <= $#words; i++ )); do
    if [[ ${words[i]} == "--index" && -n ${words[i+1]} ]]; then
      out=(--index "${words[i+1]}")
      break
    fi
  done

  print -r -- "${out[@]}"
}

_tfidf_index_files() {
  _files -g "*.index(-.)" "$@" || _files "$@"
}

_tfidf_known_top_values() {
  local -a values
  values=(
    "5:top 5 results"
    "10:top 10 results"
    "20:top 20 results"
    "50:top 50 results"
    "100:top 100 results"
    "200:top 200 results"
    "500:top 500 results"
    "1000:top 1000 results"
  )
  _describe -t top-values "top count" values
}

_tfidf_indexed_documents() {
  local -a index_args docs expl
  local line

  index_args=("${(@f)$(_tfidf_index_args_from_words)}")
  docs=()

  while IFS=$'\n' read -r line; do
    [[ -n "$line" ]] && docs+=("$line")
  done < <(command tfidf query "${index_args[@]}" list 2>/dev/null)

  if (( ${#docs} )); then
    _wanted indexed-documents expl "indexed document" compadd "$expl[@]" -a docs
  else
    _files
  fi
}

_tfidf_query_subcommands() {
  local -a subcommands
  subcommands=(
    "list:List all indexed documents"
    "terms:Show top terms"
    "similar:Find documents similar to an indexed file"
    "search:Search by query string"
  )
  _describe -t commands "query subcommand" subcommands
}

_tfidf_save() {
  _arguments -C \
    '--index[write index to path]:index file:_tfidf_index_files' \
    '*:input path:_files'
}

_tfidf_query_list() {
  _arguments -C \
    '--index[read index from path]:index file:_tfidf_index_files'
}

_tfidf_query_terms() {
  _arguments -C \
    '--index[read index from path]:index file:_tfidf_index_files' \
    '--top[show top N terms]:count:_tfidf_known_top_values'
}

_tfidf_query_similar() {
  _arguments -C \
    '--index[read index from path]:index file:_tfidf_index_files' \
    '--top[show top N similar documents]:count:_tfidf_known_top_values' \
    '1:indexed document:_tfidf_indexed_documents'
}

_tfidf_query_search() {
  _arguments -C \
    '--index[read index from path]:index file:_tfidf_index_files' \
    '--top[show top N search results]:count:_tfidf_known_top_values' \
    '1:query string:'
}

_tfidf_query() {
  local subcmd
  local -a normal_words

  normal_words=("${words[@]}")

  if (( CURRENT == 2 )); then
    _arguments -C \
      '--index[read index from path]:index file:_tfidf_index_files' \
      '1:query subcommand:_tfidf_query_subcommands'
    return
  fi

  subcmd=""
  for word in "${words[@]:2}"; do
    case "$word" in
      list|terms|similar|search)
        subcmd="$word"
        break
        ;;
    esac
  done

  if [[ -z "$subcmd" ]]; then
    _arguments -C \
      '--index[read index from path]:index file:_tfidf_index_files' \
      '1:query subcommand:_tfidf_query_subcommands'
    return
  fi

  case "$subcmd" in
    list)
      _tfidf_query_list
      ;;
    terms)
      _tfidf_query_terms
      ;;
    similar)
      _tfidf_query_similar
      ;;
    search)
      _tfidf_query_search
      ;;
  esac
}

_tfidf() {
  local context state line
  typeset -A opt_args

  local -a commands
  commands=(
    "save:Build and save an index from files and directories"
    "query:Query a saved index"
  )

  _arguments -C \
    '(-h --help)'{-h,--help}'[show help]' \
    '1:command:_describe -t commands "tfidf command" commands' \
    '*::argument:->args'

  case "$words[2]" in
    save)
      _tfidf_save
      ;;
    query)
      _tfidf_query
      ;;
  esac
}

_tfidf "$@"
