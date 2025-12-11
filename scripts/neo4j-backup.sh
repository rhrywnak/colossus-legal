#!/usr/bin/env bash
#
# backup.sh - Backup Neo4j (Community Edition) running in a Podman container using neo4j-admin dump.
#
# - Runs neo4j-admin database dump inside the container
# - Copies the resulting dump file to the host
# - Names it with a timestamp
# - Prunes backups older than N days
#
# Run this from the host (not inside the container).
#

set -euo pipefail

########################################
# CONFIGURATION
########################################

# Container runtime: podman (you can change to docker if you prefer)
RUNTIME="podman"

# Name of the running Neo4j container (yours)
NEO4J_CONTAINER_NAME="colossus-neo4j"

# Name of the database to dump (default is "neo4j")
NEO4J_DB_NAME="neo4j"

# Directory inside the container to write the dump file to
CONTAINER_BACKUP_DIR="/backups"

# Directory on the host where backups will be stored
HOST_BACKUP_DIR="/opt/neo4j_backups"

# Number of days to keep backups (remove older ones). Set to 0 to disable pruning.
RETENTION_DAYS=30


########################################
# HELPER FUNCTIONS
########################################

log() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

err() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*" >&2
}


########################################
# PRE-FLIGHT CHECKS
########################################

# Check runtime is available
if ! command -v "${RUNTIME}" >/dev/null 2>&1; then
  err "'${RUNTIME}' command not found. Ensure Podman is installed and in PATH."
  exit 1
fi

# Check container is running
if ! ${RUNTIME} ps --format '{{.Names}}' | grep -q "^${NEO4J_CONTAINER_NAME}$"; then
  err "Container '${NEO4J_CONTAINER_NAME}' is not running. Start it first or adjust NEO4J_CONTAINER_NAME."
  exit 1
fi

# Ensure host backup directory exists
if [ ! -d "${HOST_BACKUP_DIR}" ]; then
  log "Host backup directory '${HOST_BACKUP_DIR}' does not exist. Creating it..."
  mkdir -p "${HOST_BACKUP_DIR}"
fi


########################################
# BACKUP PROCESS
########################################

TIMESTAMP="$(date +'%Y-%m-%d_%H-%M-%S')"
DUMP_BASENAME="${NEO4J_DB_NAME}_${TIMESTAMP}.dump"
CONTAINER_DUMP_PATH="${CONTAINER_BACKUP_DIR}/${NEO4J_DB_NAME}.dump"
HOST_DUMP_PATH="${HOST_BACKUP_DIR}/${DUMP_BASENAME}"

log "Starting Neo4j backup for database '${NEO4J_DB_NAME}' from container '${NEO4J_CONTAINER_NAME}'."

# 1. Run neo4j-admin dump inside the container
log "Creating dump inside container at '${CONTAINER_DUMP_PATH}'..."
${RUNTIME} exec "${NEO4J_CONTAINER_NAME}" \
  neo4j-admin database dump "${NEO4J_DB_NAME}" \
  --to="${CONTAINER_BACKUP_DIR}" \
  --overwrite-destination=true

log "Dump created inside container."

# 2. Copy dump file from container to host with timestamped name
log "Copying dump from container to host: '${HOST_DUMP_PATH}'..."
${RUNTIME} cp "${NEO4J_CONTAINER_NAME}:${CONTAINER_DUMP_PATH}" "${HOST_DUMP_PATH}"

log "Backup copied to host: ${HOST_DUMP_PATH}"

# 3. (Optional) Prune old backups
if [ "${RETENTION_DAYS}" -gt 0 ]; then
  log "Pruning backups older than ${RETENTION_DAYS} days in '${HOST_BACKUP_DIR}'..."
  find "${HOST_BACKUP_DIR}" -type f -name "${NEO4J_DB_NAME}_*.dump" -mtime +${RETENTION_DAYS} -print -delete || true
fi

log "Backup completed successfully."

exit 0

