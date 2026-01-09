#!/usr/bin/env sh
echo "vlist"
echo "-----"
vlist ctx:tfidf:todo export
echo
echo "codeforge"
echo "---------"
codeforge issues tfidf-rs --json

