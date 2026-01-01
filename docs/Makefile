.PHONY: docs

MDBOOK ?= mdbook 
SRC_DIR ?= ./src
OUT_DIR ?= ./book
BOOK_TOML ?= ./book.toml

SRC_FILES := $(shell find $(SRC_DIR) -type f -name '*.md') $(BOOK_TOML)

.PHONY: all build serve watch clean

all:  $(OUT_DIR)/index.html

build: 
	$(MDBOOK) build

watch:
	find $(SRC_DIR) -type f -name '*.md' | entr -c $(MAKE) build

clean:
	rm -rf  -- $(OUT_DIR)

$(OUT_DIR)/%.html: $(SRC_FILES)
		$(MDBOOK) build

# make it so I can use m4 via -D_TEMPLATE_, so I can 
# have a template heredoc, that provides the 
# MDBOOK_PORT and HOST, at a configurable location,
# in xdg compliance,
# so that caddy can provide reverse proxy
