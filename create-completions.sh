#!/usr/bin/env sh
. $(pwd)/tai.env
mkdir -p $(pwd)/completions
terminal-ai --model=gpt-5.5 -o NO_USER_STDOUT,CONTENT_ONLY,NO_USER_ARTIFACT --output-file=.generated/complete-fish-$(uuidgen -7) --input-file=Cargo.toml --input-file=src/main.rs -- "create fish completions, provide the capability to provide parameter completions for known values" | tee completions/tfidf.fish.new
terminal-ai --model=gpt-5.5 -o NO_USER_STDOUT,CONTENT_ONLY,NO_USER_ARTIFACT --output-file=.generated/complete-zsh-$(uuidgen -7) --input-file=Cargo.toml --input-file=src/main.rs -- "create zsh completions, provide the capability to provide parameter completions for known values" | tee completions/tfidf.zsh.new
terminal-ai --model=gpt-5.5 -o NO_USER_STDOUT,CONTENT_ONLY,NO_USER_ARTIFACT --output-file=.generated/complete-bash-$(uuidgen -7) --input-file=Cargo.toml --input-file=src/main.rs -- "create bash completions, provide the capability to provide parameter completions for known values" | tee completions/tfidf.bash.new
