#!/usr/bin/env sh
TAI_MODEL=gpt-5.6-sol
. $(pwd)/tai.env
mkdir -p $(pwd)/man
terminal-ai --model=$TAI_MODEL -o NO_USER_STDOUT,CONTENT_ONLY,NO_USER_ARTIFACT --output-file=.generated/man-$(uuidgen -7) --input-file=Cargo.toml --input-file=src/main.rs -- "generate a tfidf(1) man artifact covering environment variables, synopsis, defaults, examples, and other relevant section" | tee man/tfidf.1.new
