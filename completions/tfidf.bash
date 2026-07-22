# bash completion for tfidf
#
# Install:
#   source ./tfidf.bash
# or copy to:
#   /usr/share/bash-completion/completions/tfidf
#
# Optional known-value hooks:
#   TFIDF_COMPLETE_TOP_VALUES="5 10 20 50 100"
#   TFIDF_COMPLETE_INDEX_VALUES="$HOME/.local/share/tfidf/tfidf.index ./docs.index"
#   TFIDF_COMPLETE_QUERY_VALUES="error handling rust|memory management|cosine similarity"
#   TFIDF_COMPLETE_SIMILAR_VALUES="src/main.rs README.md"
#
# Optional custom provider:
#   _tfidf_complete_values() {
#     # $1 = parameter name: index|top|similar|search-query|save-path
#     # $2 = current word
#     # print one completion candidate per line
#     case "$1" in
#       top) printf '%s\n' 5 10 20 50 100 ;;
#     esac
#   }

_tfidf_comp_words_by_ref() {
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD - 1]}"
}

_tfidf_have_filedir() {
  declare -F _filedir >/dev/null 2>&1
}

_tfidf_filedir() {
  if _tfidf_have_filedir; then
    _filedir "$@"
  else
    COMPREPLY+=($(compgen -f -- "$cur"))
  fi
}

_tfidf_compgen_lines() {
  local candidate
  while IFS= read -r candidate; do
    [[ -n "$candidate" ]] || continue
    [[ "$candidate" == "$cur"* ]] && COMPREPLY+=("$candidate")
  done
}

_tfidf_known_values() {
  local param="$1"
  local values_var="$2"
  local values="${!values_var-}"

  if declare -F _tfidf_complete_values >/dev/null 2>&1; then
    _tfidf_complete_values "$param" "$cur" | _tfidf_compgen_lines
    return
  fi

  if [[ -n "$values" ]]; then
    local old_ifs="$IFS"
    IFS='|'
    local v
    for v in $values; do
      [[ "$v" == "$cur"* ]] && COMPREPLY+=("$v")
    done
    IFS="$old_ifs"

    if ((${#COMPREPLY[@]})); then
      return
    fi

    COMPREPLY+=($(compgen -W "$values" -- "$cur"))
  fi
}

_tfidf_index_path_from_words() {
  local i
  for ((i = 1; i < COMP_CWORD; i++)); do
    if [[ "${COMP_WORDS[i]}" == "--index" && $((i + 1)) -lt COMP_CWORD ]]; then
      printf '%s\n' "${COMP_WORDS[i + 1]}"
      return
    fi
  done

  if [[ -n "${TFIDF_INDEX-}" ]]; then
    printf '%s\n' "$TFIDF_INDEX"
    return
  fi
}

_tfidf_query_list_command() {
  local index_path
  index_path="$(_tfidf_index_path_from_words)"

  if [[ -n "$index_path" ]]; then
    command tfidf query --index "$index_path" list 2>/dev/null
  else
    command tfidf query list 2>/dev/null
  fi
}

_tfidf_complete_index() {
  _tfidf_known_values index TFIDF_COMPLETE_INDEX_VALUES
  _tfidf_filedir
}

_tfidf_complete_top() {
  if declare -F _tfidf_complete_values >/dev/null 2>&1 || [[ -n "${TFIDF_COMPLETE_TOP_VALUES-}" ]]; then
    _tfidf_known_values top TFIDF_COMPLETE_TOP_VALUES
  else
    COMPREPLY+=($(compgen -W "5 10 20 50 100 200 500 1000" -- "$cur"))
  fi
}

_tfidf_complete_similar_target() {
  if declare -F _tfidf_complete_values >/dev/null 2>&1 || [[ -n "${TFIDF_COMPLETE_SIMILAR_VALUES-}" ]]; then
    _tfidf_known_values similar TFIDF_COMPLETE_SIMILAR_VALUES
  else
    _tfidf_query_list_command | _tfidf_compgen_lines
  fi

  _tfidf_filedir
}

_tfidf_complete_search_query() {
  _tfidf_known_values search-query TFIDF_COMPLETE_QUERY_VALUES
}

_tfidf_complete_save_path() {
  if declare -F _tfidf_complete_values >/dev/null 2>&1 || [[ -n "${TFIDF_COMPLETE_SAVE_PATH_VALUES-}" ]]; then
    _tfidf_known_values save-path TFIDF_COMPLETE_SAVE_PATH_VALUES
  fi

  _tfidf_filedir
}

_tfidf_seen_word() {
  local needle="$1"
  local i
  for ((i = 1; i < COMP_CWORD; i++)); do
    [[ "${COMP_WORDS[i]}" == "$needle" ]] && return 0
  done
  return 1
}

_tfidf_query_subcommand_index() {
  local i
  for ((i = 2; i < COMP_CWORD; i++)); do
    case "${COMP_WORDS[i]}" in
    --index)
      ((i++))
      ;;
    list | terms | similar | search)
      printf '%s\n' "$i"
      return
      ;;
    esac
  done
}

_tfidf_complete_save() {
  case "$prev" in
  --index)
    _tfidf_complete_index
    return
    ;;
  esac

  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--index --" -- "$cur"))
    return
  fi

  _tfidf_complete_save_path
}

_tfidf_complete_query_terms() {
  case "$prev" in
  --top)
    _tfidf_complete_top
    return
    ;;
  esac

  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--top" -- "$cur"))
  fi
}

_tfidf_complete_query_similar() {
  case "$prev" in
  --top)
    _tfidf_complete_top
    return
    ;;
  esac

  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--top" -- "$cur"))
    return
  fi

  local subidx
  subidx="$(_tfidf_query_subcommand_index)"

  local target_seen=0
  local i
  for ((i = subidx + 1; i < COMP_CWORD; i++)); do
    case "${COMP_WORDS[i]}" in
    --top)
      ((i++))
      ;;
    --*)
      ;;
    *)
      target_seen=1
      ;;
    esac
  done

  if ((target_seen == 0)); then
    _tfidf_complete_similar_target
  fi
}

_tfidf_complete_query_search() {
  case "$prev" in
  --top)
    _tfidf_complete_top
    return
    ;;
  esac

  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--top" -- "$cur"))
    return
  fi

  _tfidf_complete_search_query
}

_tfidf_complete_query() {
  case "$prev" in
  --index)
    _tfidf_complete_index
    return
    ;;
  esac

  local subidx subcmd
  subidx="$(_tfidf_query_subcommand_index)"

  if [[ -z "$subidx" ]]; then
    if [[ "$cur" == -* ]]; then
      COMPREPLY=($(compgen -W "--index" -- "$cur"))
    else
      COMPREPLY=($(compgen -W "list terms similar search" -- "$cur"))
    fi
    return
  fi

  subcmd="${COMP_WORDS[subidx]}"

  case "$subcmd" in
  list)
    ;;
  terms)
    _tfidf_complete_query_terms
    ;;
  similar)
    _tfidf_complete_query_similar
    ;;
  search)
    _tfidf_complete_query_search
    ;;
  esac
}

_tfidf() {
  local cur prev
  COMPREPLY=()
  _tfidf_comp_words_by_ref

  case "$prev" in
  --index)
    _tfidf_complete_index
    return 0
    ;;
  --top)
    _tfidf_complete_top
    return 0
    ;;
  esac

  if ((COMP_CWORD == 1)); then
    if [[ "$cur" == -* ]]; then
      COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
    else
      COMPREPLY=($(compgen -W "save query" -- "$cur"))
    fi
    return 0
  fi

  case "${COMP_WORDS[1]}" in
  save)
    _tfidf_complete_save
    ;;
  query)
    _tfidf_complete_query
    ;;
  *)
    COMPREPLY=($(compgen -W "save query -h --help" -- "$cur"))
    ;;
  esac

  return 0
}

complete -o bashdefault -o default -o nospace -F _tfidf tfidf
