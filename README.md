# hospital_register

A production-grade Hospital Register API built with Axum, SQLx, and PostgreSQL.

## Stack

- **Runtime:** Tokio
- **Web:** Axum 0.8
- **Database:** PostgreSQL 16 (via SQLx, no ORM)
- **Cache:** Redis 7
- **Observability:** tracing + Prometheus

## Local Development

### Prerequisites

- Rust 1.85+ (pinned via `rust-toolchain.toml`)
- Docker + Docker Compose
- `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features postgres,rustls`

### Start services

```bash
make up