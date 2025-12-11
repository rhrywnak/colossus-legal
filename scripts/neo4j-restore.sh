#!/usr/bin/env bash
#
# restore.sh - Restore Neo4j (Community Edition) running in a Podman container
#              from a dump created by neo4j-admin (e.g., with backup.sh).
#
# Usage:
#   ./restore.sh /opt/neo4j_backups/neo4j_YYYY-MM-DD_HH-MM-SS.dump
#
# This will:
#   - Stop the main Neo4j container
#   - Run a temporary neo4j-admin load using the same data volume
#   - Restart the main container
#

set -euo pipefail

########################################
# CONFIGURATION
########################################

# Container runtime
RUNTIME="podman"

# Name of the running Neo4j container
NEO4J_CONTAINER_NAME="colossus-neo4j"

# Name of the database to restore
NEO4J_DB_NAME="neo4j"

# Neo4j image to use for the temporary restore container
# (Should match your major version; adjust tag if needed.)
NEO4J_IMAGE="neo4j:5"

# Host directory where backup dumps are stored
HOST_BACKUP_DIR="/opt/neo4j_backups"

# Name for the temporary restore container
RESTORE_CONTAINER_NAME="neo4j-restore-temp"


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
# ARGUMENTS
########################################

if [ $# -lt 1 ]; then
  err "No backup file specified."
  echo "Usage: $0 ${HOST_BACKUP_DIR}/neo4j_YYYY-MM-DD_HH-MM-SS.dump"
  exit 1
fi

BACKUP_FILE="$1"

if [ ! -f "${BACKUP_FILE}" ]; then
  err "Backup file '${BACKUP_FILE}' does not exist."
  exit 1
fi

BACKUP_DIR="$(dirname "${BACKUP_FILE}")"
BACKUP_BASENAME="$(basename "${BACKUP_FILE}")"


########################################
# PRE-FLIGHT CHECKS
########################################

# Check runtime is available
if ! command -v "${RUNTIME}" >/dev/null 2>&1; then
  err "'${RUNTIME}' command not found. Ensure Podman is installed and in PATH."
  exit 1
fi

# Check container exists (running or not)
if ! ${RUNTIME} ps -a --format '{{.Names}}' | grep -q "^${NEO4J_CONTAINER_NAME}$"; then
  err "Container '${NEO4J_CONTAINER_NAME}' does not exist. Adjust NEO4J_CONTAINER_NAME or create the container first."
  exit 1
fi

# Check the image exists locally (optional)
if ! ${RUNTIME} images --format '{{.Repository}}:{{.Tag}}' | grep -q "^${NEO4J_IMAGE}$"; then
  log "Image '${NEO4J_IMAGE}' not found locally. Pulling..."
  ${RUNTIME} pull "${NEO4J_IMAGE}"
fi


########################################
# RESTORE PROCESS
########################################

log "Starting restore of database '${NEO4J_DB_NAME}' from backup '${BACKUP_FILE}'."

# 1. Stop the main Neo4j container if running
if ${RUNTIME} ps --format '{{.Names}}' | grep -q "^${NEO4J_CONTAINER_NAME}$"; then
  log "Stopping container '${NEO4J_CONTAINER_NAME}'..."
  ${RUNTIME} stop "${NEO4J_CONTAINER_NAME}"
else
  log "Container '${NEO4J_CONTAINER_NAME}' is not running. Continuing with restore."
fi

# 2. Remove any previous temp restore container if it exists
if ${RUNTIME} ps -a --format '{{.Names}}' | grep -q "^${RESTORE_CONTAINER_NAME}$"; then
  log "Removing existing temporary restore container '${RESTORE_CONTAINER_NAME}'..."
  ${RUNTIME} rm -f "${RESTORE_CONTAINER_NAME}" || true
fi

# 3. Run neo4j-admin load in a temporary container that:
#    - Shares the same data volume as the main container (via --volumes-from)
#    - Mounts the host backup directory at /backup
log "Running neo4j-admin database load in temporary container '${RESTORE_CONTAINER_NAME}'..."

${RUNTIME} run --rm \
  --name "${RESTORE_CONTAINER_NAME}" \
  --volumes-from "${NEO4J_CONTAINER_NAME}" \
  -v "${BACKUP_DIR}":/backup \
  "${NEO4J_IMAGE}" \
  neo4j-admin database load "${NEO4J_DB_NAME}" \
    --from="/backup/${BACKUP_BASENAME}" \
    --force

log "Database '${NEO4J_DB_NAME}' loaded from backup."

# 4. Restart the main Neo4j container
log "Starting container '${NEO4J_CONTAINER_NAME}'..."
${RUNTIME} start "${NEO4J_CONTAINER_NAME}"

log "Restore completed successfully."
log "Neo4j should now be running with data from '${BACKUP_BASENAME}'."

exit 0

