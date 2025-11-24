# Repository Guidelines

## Project Structure & Module Organization
- `backend/`: Rust Axum API; `src/` holds routes, config, and Neo4j integration. Keep new modules small and place shared types in `state`/`config` style modules.
- `frontend/`: React + Vite + TypeScript UI. Pages live in `src/pages/`, shared services in `src/services/`, styling in `src/styles/`.
- `docs/` for architecture/API/model references; `prompts/` for LLM templates; `scripts/` for helper scripts (bootstrap, etc.); `docker-compose.yml` for local containers.

## Build, Test, and Development Commands
- `make backend` → `cargo run` in `backend/`; `make frontend` → `npm run dev` in `frontend/`; `make dev` prints the two-terminal flow.
- Backend tests: `cd backend && cargo test`.
- Frontend check: `cd frontend && npm run build` (Vite build + TypeScript typecheck). `npm run dev` for hot reload on port 5473.
- Dockerized stack: `docker compose up --build`; stop with `docker compose down`; stream logs via `docker compose logs -f` or `make logs`.

## Coding Style & Naming Conventions
- Rust: run `cargo fmt` and `cargo clippy -- -D warnings` before PRs. Modules/files snake_case; types and enums PascalCase; functions and variables snake_case. Prefer explicit structs for responses and use `tracing` for logs.
- Frontend: 2-space indentation; components PascalCase; hooks/utilities camelCase; route paths and files lower-case-kebab where applicable. Keep components functional and colocate lightweight helpers near usage.
- Env vars live in `.env`; do not hardcode secrets in code or commits.

## Testing Guidelines
- Backend: add `#[cfg(test)]` modules next to the code they verify; integration-style tests can live under `backend/tests/`. Cover new endpoints and Neo4j interactions; mock external calls when feasible.
- Frontend: no test harness yet—gate changes by exercising flows in the dev server and ensuring `npm run build` stays clean. If you add Vitest/RTL, place specs alongside components with `.test.tsx` suffix.

## Commit & Pull Request Guidelines
- Commits: short, imperative titles (e.g., `Add claims router`, `Fix Neo4j health check`); keep scope focused.
- PRs: include what changed, why, and how to verify. Link tickets; attach screenshots or terminal output for UI/API changes. Confirm `cargo test` and `npm run build` (or `docker compose up --build` if relevant) before requesting review.

## Security & Configuration Tips
- Escape `$` in `.env` values as `\$` to avoid shell interpolation (e.g., `NEO4J_PASSWORD=Drwho2010\$`). Keep `.env.example` updated when adding new settings.
- Never commit secrets or Neo4j credentials; prefer `.env` + `dotenvy`/Vite env loading.
