# JMDict data URL and file paths
JMDICT_VERSION = 3.6.1+20250818123231
JMDICT_URL = https://github.com/scriptin/jmdict-simplified/releases/download/$(JMDICT_VERSION)/jmdict-eng-$(JMDICT_VERSION).json.tgz
JMDICT_FILE = jmdict-codegen/assets/jmdict-eng-$(JMDICT_VERSION).json.tgz

# Download JMDict data if not present
fetch-jmdict:
	@echo "Checking for JMDict data..."
	@if [ ! -f "$(JMDICT_FILE)" ]; then \
		echo "Downloading JMDict data from $(JMDICT_URL)"; \
		mkdir -p jmdict-codegen/assets; \
		curl -L -o "$(JMDICT_FILE)" "$(JMDICT_URL)"; \
		echo "JMDict data downloaded successfully"; \
	else \
		echo "JMDict data already exists"; \
	fi

codegen: fetch-jmdict
	cd jmdict-codegen && cargo run && cargo clean
	@echo "Dictionary data generated successfully"

codegen-test: fetch-jmdict
	cd jmdict-codegen && cargo run -- --limit 1000 && cargo clean

codegen-web: fetch-jmdict
	cd jmdict-codegen && CARGO_CFG_TARGET_ARCH=wasm32 cargo run && cargo clean
	@echo "Web-optimized dictionary data generated successfully"

# Build dictionary data (rarely needed)
dict-data:
	cd dictionary-data && cargo build --profile dict && cargo clean

# Check and generate dictionary data if needed
check-dict-data:
	@if [ ! -f dictionary-data/src/lib.rs ]; then \
		echo "Dictionary data not found, generating..."; \
		$(MAKE) codegen; \
	fi

# TUI version
tui: check-dict-data
	cd japandict-tui && cargo run --release -- --tui

# Web version  
web: check-dict-data
	cd japandict-web && dx serve --platform web

# Build web for production
web-build: check-dict-data
	cd japandict-web && dx build --platform web

# Help target
help:
	@echo "Available targets:"
	@echo "  fetch-jmdict  - Download JMDict data from scriptin/jmdict-simplified"
	@echo "  codegen       - Generate full dictionary data (213K words)"
	@echo "  codegen-web   - Generate web-optimized dictionary data (15K common words)"
	@echo "  codegen-test  - Generate test dictionary data (1K words)"
	@echo "  tui           - Run TUI application"
	@echo "  web           - Run web development server"
	@echo "  web-build     - Build web application for production"
	@echo "  clean         - Clean all build artifacts"
	@echo "  clean-data    - Clean dictionary data and force regeneration"

clean:
	# Clean main workspace
	cargo clean
	# Clean excluded dictionary-data crate
	cd dictionary-data && cargo clean
	# Clean jmdict-codegen with its own target dir
	cd jmdict-codegen && cargo clean

clean-data:
	rm -f dictionary-data/src/lib.rs
	@echo "Dictionary data removed. Run 'make codegen' to regenerate."

.PHONY: help fetch-jmdict codegen codegen-web codegen-test tui web web-build clean clean-data check-dict-data dict-data
