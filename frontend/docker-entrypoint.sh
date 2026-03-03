#!/bin/sh
# ==============================================================================
# Runtime Config Injection
# ==============================================================================
# This script runs BEFORE nginx starts. It writes a config.js file that the
# React app loads at startup, providing environment-specific values without
# needing separate builds per environment.
#
# The React app includes <script src="/config.js"> in index.html, which sets
# window.__COLOSSUS_CONFIG__. Components read from that instead of
# import.meta.env.VITE_API_URL.
# ==============================================================================

# Default to localhost if not set (useful for local development)
API_URL="${COLOSSUS_API_URL:-http://localhost:3403}"
AUTH_LOGOUT_URL="${COLOSSUS_AUTH_LOGOUT_URL:-https://auth.cogmai.com/application/o/colossus-services/end-session/}"

# Write the runtime config file
cat > /usr/share/nginx/html/config.js <<EOF
// Auto-generated at container startup — do not edit
window.__COLOSSUS_CONFIG__ = {
  apiUrl: "${API_URL}",
  authLogoutUrl: "${AUTH_LOGOUT_URL}"
};
EOF

echo "Runtime config written: API_URL=${API_URL}, AUTH_LOGOUT_URL=${AUTH_LOGOUT_URL}"

# Execute the CMD (nginx)
exec "$@"
