#!/usr/bin/env bash

if [ "$1" = "help" ]; then
  printf "%s\n" "=== ./install.sh Local TFIDF Installer ==="
  printf "%s\n" "Symlinks 'tfidf' release to $HOME/.local/bin/tfidf."
  printf "%s\n" "Install to '/usr/local/bin/tfidf' with 'sudo ./install.sh release"
  exit
fi

ln -sf ./target/release/tfidf $HOME/.local/bin

if [ "$1" = "release" ]; then
  cp ./target/release/tfidf /usr/local/bin/tfidf
fi
