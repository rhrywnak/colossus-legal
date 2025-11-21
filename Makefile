.PHONY: all backend frontend dev up down logs

all: dev

backend:
	cd backend && cargo run

frontend:
	cd frontend && npm run dev

dev:
	@echo "Start backend and frontend separately:"
	@echo "  terminal 1: make backend"
	@echo "  terminal 2: make frontend"

up:
	docker compose up -d

down:
	docker compose down

logs:
	docker compose logs -f
