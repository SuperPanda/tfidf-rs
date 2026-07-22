#!/usr/bin/env sh
. .env
mkdir -p man
terminal-ai --model=gpt-5.5 -oNO_USER_STDOUT,CONTENT_ONLY --output-file=.generated/man-$(uuidgen -7) --input-file=Cargo.toml --input-file=src/main.rs -- "generate a tfidf(1) man artifact covering environment variables, synopsis, defaults, examples, and other relevant section" | tee man/tfidf.1.man
