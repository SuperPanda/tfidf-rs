#!/usr/bin/env bash
ln -sf ./target/release/tfidf $HOME/.local/bin

if [ "$1" = "release" ]; then
  cp ./target/release/tfidf /usr/local/bin/tfidf
fi
