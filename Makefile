# Deploy shortcuts. Everything real lives in deploy/ — see deploy/config.sh for the
# server settings, all overridable from the environment:
#
#   make deploy SSH_HOST=ced@other-box
#
.PHONY: help run test setup nginx deploy rollback seed logs status versions

help:  ## Show this help
	@grep -E '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk -F':.*?## ' '{printf "  %-10s %s\n", $$1, $$2}'

run:  ## Serve the app locally on http://127.0.0.1:5000
	@cargo run

test:  ## Run the test suite and the linter locally
	@cargo test
	@cargo clippy --all-targets -- -D warnings

setup:  ## One-time server preparation (directories, systemd check, nginx config)
	@./deploy/setup_server.sh

nginx:  ## Re-install the nginx snippet and reload nginx (no deploy)
	@./deploy/nginx.sh

deploy:  ## Build, ship and start the new version, then health check it
	@./deploy/deploy.sh

rollback:  ## Switch back to the previously deployed version
	@./deploy/rollback.sh

seed:  ## Fill the server's catalog from the release's JSON (no page parsing)
	@./deploy/seed.sh

logs:  ## Follow the app's logs (Ctrl-C to stop)
	@./deploy/server.sh logs

status:  ## Running unit + /health, as seen from the server
	@./deploy/server.sh status

versions:  ## List the versions kept on the server
	@./deploy/server.sh versions
