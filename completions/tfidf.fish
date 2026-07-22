complete -c tfidf -e

function __fish_tfidf_primary
    set -l cmd (commandline -opc)

    for tok in $cmd[2..-1]
        switch $tok
            case save query
                printf '%s\n' "$tok"
                return 0
        end
    end

    return 1
end

function __fish_tfidf_command_is --argument-names wanted
    set -l primary (__fish_tfidf_primary)
    test "$primary" = "$wanted"
end

function __fish_tfidf_needs_primary
    not __fish_tfidf_primary >/dev/null
end

function __fish_tfidf_completing_option_argument
    set -l cmd (commandline -opc)

    test (count $cmd) -gt 1; or return 1

    switch $cmd[-1]
        case --index --top
            return 0
    end

    return 1
end

function __fish_tfidf_query_subcommand
    __fish_tfidf_command_is query; or return 1

    set -l cmd (commandline -opc)
    set -l after_query 0
    set -l skip_next 0

    for tok in $cmd[2..-1]
        if test $skip_next -eq 1
            set skip_next 0
            continue
        end

        if test $after_query -eq 0
            if test "$tok" = query
                set after_query 1
            end
            continue
        end

        switch $tok
            case --index
                set skip_next 1
            case list terms similar search
                printf '%s\n' "$tok"
                return 0
            case '-*'
            case '*'
                return 1
        end
    end

    return 1
end

function __fish_tfidf_query_subcommand_is --argument-names wanted
    set -l subcmd (__fish_tfidf_query_subcommand)
    test "$subcmd" = "$wanted"
end

function __fish_tfidf_query_needs_subcommand
    __fish_tfidf_command_is query; or return 1
    not __fish_tfidf_query_subcommand >/dev/null
end

function __fish_tfidf_query_similar_needs_target
    __fish_tfidf_query_subcommand_is similar; or return 1

    set -l cmd (commandline -opc)
    set -l after_similar 0
    set -l skip_next 0

    for tok in $cmd[2..-1]
        if test $skip_next -eq 1
            set skip_next 0
            continue
        end

        if test $after_similar -eq 0
            if test "$tok" = similar
                set after_similar 1
            end
            continue
        end

        switch $tok
            case --index --top
                set skip_next 1
            case '-*'
            case '*'
                return 1
        end
    end

    test $after_similar -eq 1
end

function __fish_tfidf_query_similar_has_target
    __fish_tfidf_query_subcommand_is similar; or return 1
    not __fish_tfidf_query_similar_needs_target
end

function __fish_tfidf_emit_completion --argument-names value description
    test -n "$value"; or return 1
    printf '%s\t%s\n' (string escape -- "$value") "$description"
end

function __fish_tfidf_known_index_paths
    set -l app tfidf

    if set -q TFIDF_APPNAME; and test -n "$TFIDF_APPNAME"
        set app "$TFIDF_APPNAME"
    end

    if set -q TFIDF_INDEX; and test -n "$TFIDF_INDEX"
        __fish_tfidf_emit_completion "$TFIDF_INDEX" TFIDF_INDEX
    end

    if set -q TFIDF_DATA_DIR; and test -n "$TFIDF_DATA_DIR"
        __fish_tfidf_emit_completion "$TFIDF_DATA_DIR/tfidf.index" 'TFIDF_DATA_DIR default'
    else if set -q XDG_DATA_HOME; and test -n "$XDG_DATA_HOME"
        __fish_tfidf_emit_completion "$XDG_DATA_HOME/$app/tfidf.index" 'XDG data default'
    else if set -q HOME; and test -n "$HOME"
        __fish_tfidf_emit_completion "$HOME/.local/share/$app/tfidf.index" 'HOME data default'
    else
        __fish_tfidf_emit_completion "$PWD/$app/tfidf.index" 'current directory fallback'
    end
end

function __fish_tfidf_index_args
    set -l cmd (commandline -opc)
    set -l i 2

    while test $i -le (count $cmd)
        if test "$cmd[$i]" = --index
            set -l next (math $i + 1)
            if set -q cmd[$next]
                printf '%s\n' --index
                printf '%s\n' "$cmd[$next]"
            end
            return 0
        end

        set i (math $i + 1)
    end
end

function __fish_tfidf_indexed_documents
    set -l index_args (__fish_tfidf_index_args)

    command tfidf query $index_args list 2>/dev/null | while read -l path
        __fish_tfidf_emit_completion "$path" 'indexed document'
    end
end

function __fish_tfidf_top_values
    printf '%s\t%s\n' 1 'top 1 result'
    printf '%s\t%s\n' 5 'top 5 results'
    printf '%s\t%s\n' 10 'top 10 results'
    printf '%s\t%s\n' 20 'top 20 results'
    printf '%s\t%s\n' 50 'top 50 results'
    printf '%s\t%s\n' 100 'top 100 results'
    printf '%s\t%s\n' 500 'top 500 results'
    printf '%s\t%s\n' 1000 'top 1000 results'
end

complete -c tfidf -f -s h -l help -d 'Show help'

complete -c tfidf -f -n "__fish_tfidf_needs_primary; and not __fish_tfidf_completing_option_argument" -a save -d 'Build and save a TF-IDF index'
complete -c tfidf -f -n "__fish_tfidf_needs_primary; and not __fish_tfidf_completing_option_argument" -a query -d 'Query a saved TF-IDF index'

complete -c tfidf -n "__fish_tfidf_command_is save" -l index -r -F -a "(__fish_tfidf_known_index_paths)" -d 'Index path'
complete -c tfidf -n "__fish_tfidf_command_is save; and not __fish_tfidf_completing_option_argument" -a -- -d 'Read additional paths from stdin'

complete -c tfidf -n "__fish_tfidf_command_is query" -l index -r -F -a "(__fish_tfidf_known_index_paths)" -d 'Index path'

complete -c tfidf -f -n "__fish_tfidf_query_needs_subcommand; and not __fish_tfidf_completing_option_argument" -a list -d 'List all indexed documents'
complete -c tfidf -f -n "__fish_tfidf_query_needs_subcommand; and not __fish_tfidf_completing_option_argument" -a terms -d 'Show top terms'
complete -c tfidf -f -n "__fish_tfidf_query_needs_subcommand; and not __fish_tfidf_completing_option_argument" -a similar -d 'Find similar indexed documents'
complete -c tfidf -f -n "__fish_tfidf_query_needs_subcommand; and not __fish_tfidf_completing_option_argument" -a search -d 'Search indexed documents'

complete -c tfidf -f -n "__fish_tfidf_query_subcommand_is list"

complete -c tfidf -f -n "__fish_tfidf_query_subcommand_is terms"
complete -c tfidf -n "__fish_tfidf_query_subcommand_is terms" -l top -r -a "(__fish_tfidf_top_values)" -d 'Number of terms to show'

complete -c tfidf -f -n "__fish_tfidf_query_subcommand_is similar"
complete -c tfidf -f -n "__fish_tfidf_query_similar_needs_target; and not __fish_tfidf_completing_option_argument" -a "(__fish_tfidf_indexed_documents)"
complete -c tfidf -n __fish_tfidf_query_similar_has_target -l top -r -a "(__fish_tfidf_top_values)" -d 'Number of similar documents to show'

complete -c tfidf -f -n "__fish_tfidf_query_subcommand_is search"
complete -c tfidf -n "__fish_tfidf_query_subcommand_is search" -l top -r -a "(__fish_tfidf_top_values)" -d 'Number of search results to show'
