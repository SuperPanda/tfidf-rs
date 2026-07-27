#!/usr/bin/env sh
mkdir -p .generated/transcripts
export REPO_ROOT=${REPO_ROOT:-"$(git rev-parse --show-toplevel)"}
INPUT_FILES_ARGS="--input-file=$REPO_ROOT/src/main.rs"
terminal-ai --transcript-dir=$REPO_ROOT/.generated/transcripts --output-file=$REPO_ROOT/.generated/tai-message-$(uuidgen -7) --model=gpt-5.5 $INPUT_FILES_ARGS -- "${1}"
