# ==============================================================================
# Colossus-Legal Backend — Multi-Stage Dockerfile
# ==============================================================================
#
# RUST LEARNING NOTE:
# This Dockerfile uses a "multi-stage build" — a key pattern for compiled languages.
#
# Stage 1 ("builder"): Uses a full Rust toolchain image (~1.5 GB) to compile the
#   binary. This stage has cargo, rustc, linker, system headers — everything needed
#   to build. But we don't want any of that in production.
#
# Stage 2 ("runtime"): Uses a minimal Ubuntu image (~80 MB). We COPY only the
#   compiled binary from stage 1. The final image contains just:
#   - The OS with OpenSSL + CA certs (needed for TLS to Neo4j)
#   - Your single compiled Rust binary (~15-30 MB)
#
# Result: A ~100 MB production image instead of 1.5 GB.
#
# Why Rust shines here: Unlike Python/Node which need their entire runtime shipped,
# Rust compiles to a native binary with no runtime dependencies beyond libc and
# any dynamically-linked system libraries (like OpenSSL).
#
# BASE IMAGE NOTE: We use Ubuntu 24.04 (Noble) instead of Debian Bookworm because
# the ort/ort-sys crates (ONNX Runtime bindings, pulled in by fastembed) ship
# prebuilt libraries compiled against glibc 2.38+. Debian Bookworm has glibc 2.36
# which causes __isoc23_strtol linker errors. Ubuntu 24.04 has glibc 2.39,
# matching the dev workstation. Always keep builder and runtime glibc in sync.
#
# PRIVATE GIT DEPENDENCY NOTE:
# Cargo.toml references colossus-rag and colossus-auth from a private GitHub repo
# (colossus-rs). Docker builds can't access private repos without credentials.
# Instead, build-release.sh copies the colossus-rs source into the build context,
# and .cargo/config.toml overrides the git URLs to use local paths.
# This avoids putting GitHub tokens in the Docker image.
# ==============================================================================

# ── Stage 1: Build ──────────────────────────────────────────────────────────────
FROM docker.io/library/ubuntu:24.04 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain (match workstation version)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain 1.88.0
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# ── Copy colossus-rs (shared library workspace) ────────────────────────────────
# build-release.sh stages this directory before running podman build.
# It contains colossus-auth and colossus-rag source code.
COPY .colossus-rs-build/ /build/colossus-rs/

# ── Copy cargo config that redirects git deps to local paths ───────────────────
# This tells cargo: "when you see git = 'https://github.com/rhrywnak/colossus-rs.git',
# use the local /build/colossus-rs/ directory instead of fetching from GitHub."
COPY .cargo/ /build/.cargo/

# OPTIMIZATION: Copy dependency files first, then build dependencies.
# Docker caches each layer. If Cargo.toml hasn't changed, Docker reuses the
# cached dependency build — saving 5-10 minutes on rebuilds when only your
# source code changed. This is called "dependency caching."
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs so cargo can resolve and build dependencies.
# This is a common Rust Docker trick: we need a valid Rust project for cargo
# to compile dependencies, but we don't want to copy our actual source yet
# (that would bust the cache every time we change a line of code).
RUN mkdir src && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub mod config; pub mod api; pub mod dto; pub mod error; pub mod models; pub mod neo4j; pub mod repositories; pub mod services; pub mod state; pub mod auth;' > src/lib.rs && \
    mkdir -p src/api src/dto src/models src/repositories src/services && \
    touch src/api/mod.rs src/dto/mod.rs src/models/mod.rs src/repositories/mod.rs src/services/mod.rs && \
    echo 'pub struct AppConfig { pub neo4j_uri: String, pub neo4j_user: String, pub neo4j_password: String, pub document_storage_path: String, }' > src/config.rs && \
    echo 'pub struct AppState {}' > src/state.rs && \
    echo '' > src/error.rs && \
    echo '' > src/neo4j.rs && \
    echo '' > src/auth.rs

# Build dependencies only (this layer gets cached)
# NOTE: || true because the dummy source won't fully compile — we only want
# to download and compile DEPENDENCIES, not our code.
RUN cargo build --release 2>/dev/null || true

# Now copy the real source code
COPY src/ src/

# Touch main.rs to ensure cargo rebuilds our code (not the cached dummy)
RUN touch src/main.rs src/lib.rs

# Build the actual application
# NOTE: --locked removed because the local path override changes the lockfile
# resolution. The Cargo.lock in the repo references git URLs, but we're building
# with local paths. cargo will verify versions match but won't enforce exact URLs.
RUN cargo build --release

# ── Stage 2: Runtime ────────────────────────────────────────────────────────────
FROM docker.io/library/ubuntu:24.04

# Install only what the binary needs at runtime:
# - libssl3t64: OpenSSL for TLS connections (Neo4j bolt+s://)
#   (Ubuntu 24.04 renamed libssl3 → libssl3t64 for 64-bit time_t transition)
# - ca-certificates: Trust store for TLS certificate validation
# - curl: Used by HEALTHCHECK
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libssl3t64 \
        ca-certificates \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Run as non-root (security best practice)
RUN useradd --create-home --shell /bin/bash appuser
USER appuser
WORKDIR /home/appuser

# Copy the compiled binary from the builder stage
COPY --from=builder /build/target/release/colossus-legal-backend ./colossus-legal-backend

# Create document storage directory
RUN mkdir -p /home/appuser/data/documents

# Expose the backend port
EXPOSE 3403

# Health check — uses the /health endpoint
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3403/health || exit 1

# Run the backend
# RUST LEARNING NOTE: This is the entire "runtime" — a single binary.
# No interpreter, no virtual environment, no node_modules. Just the binary.
CMD ["./colossus-legal-backend"]
