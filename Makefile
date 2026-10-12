.PHONY: up down nuke ps logs migrate migrate-add test lint fmt check run clean

# --- Docker ---

up: ## Start Postgres + Redis, wait until healthy
	docker compose up -d
	@echo "Waiting for services to be healthy..."
	@until [ "$$(docker inspect -f '{{.State.Health.Status}}' hospital_postgres 2>/dev/null)" = "healthy" ] && \
	       [ "$$(docker inspect -f '{{.State.Health.Status}}' hospital_redis 2>/dev/null)" = "healthy" ]; do \
		sleep 1; \
	done
	@echo "All services healthy."

down: ## Stop containers, keep volumes
	docker compose down

seed: ## Seed a dev doctor
	cargo run --example seed_doctor

nuke: ## Stop containers and DELETE volumes (destructive)
	docker compose down -v

ps: ## Show container status
	docker compose ps

logs: ## Follow container logs
	docker compose logs -f

# --- Database ---

migrate: ## Run pending migrations
	sqlx migrate run

migrate-add: ## Create a new migration (usage: make migrate-add NAME=create_patients)
	sqlx migrate add $(NAME)

# --- Rust ---

check: ## Type-check without building
	cargo check

run: ## Run the dev server
	cargo run

test: ## Run all tests
	cargo test

lint: ## Run clippy with warnings as errors
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code
	cargo fmt --all

clean: ## Remove build artifacts
	cargo clean