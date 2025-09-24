CONTEXT_BASE_PATH="${0}"
pwd
echo "$CONTEXT_BASE_PATH"
sed 's/^/"/g;s/$/"/g' <<EOF | xargs bat --decorations=always 
AGENT.md
context.sh
tfidf-calc.rs
tfidf-explore.rs
EOF
