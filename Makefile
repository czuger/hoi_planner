.PHONY: help run test

help:  ## Show this help
	@grep -E '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk -F':.*?## ' '{printf "  %-10s %s\n", $$1, $$2}'

run:  ## Serve the app locally on http://127.0.0.1:5000
	@cargo run

test:  ## Run the test suite and the linter locally
	@cargo test
	@cargo clippy --all-targets -- -D warnings
