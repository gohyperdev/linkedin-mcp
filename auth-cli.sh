#!/bin/bash
# One-time OAuth2 authorization for linkedin-mcp
# Saves token to ~/.config/linkedin-mcp/tokens.json
set -euo pipefail

CLIENT_ID="${LINKEDIN_CLIENT_ID:?Set LINKEDIN_CLIENT_ID}"
CLIENT_SECRET="${LINKEDIN_CLIENT_SECRET:?Set LINKEDIN_CLIENT_SECRET}"
REDIRECT_URI="http://localhost:3000/callback"
SCOPES="openid%20profile%20email%20w_member_social"
STATE="$(date +%s)"

AUTH_URL="https://www.linkedin.com/oauth/v2/authorization?response_type=code&client_id=${CLIENT_ID}&redirect_uri=$(python3 -c "import urllib.parse; print(urllib.parse.quote('${REDIRECT_URI}', safe=''))")&scope=${SCOPES}&state=${STATE}"

echo ""
echo "=== LinkedIn OAuth Authorization ==="
echo "Open this URL in your browser:"
echo ""
echo "  ${AUTH_URL}"
echo ""
echo "Waiting for callback on ${REDIRECT_URI} ..."
echo ""

# Minimal HTTP server to catch the callback
RESPONSE=$(python3 -c "
import http.server, urllib.parse, json, sys

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        params = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        code = params.get('code', [None])[0]
        state = params.get('state', [None])[0]
        error = params.get('error', [None])[0]

        if error:
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'<h1>Authorization failed</h1>')
            print(json.dumps({'error': error}), file=sys.stderr)
            sys.exit(1)

        self.send_response(200)
        self.end_headers()
        self.wfile.write(b'<h1>Authorization successful!</h1><p>You can close this window.</p>')
        print(code)  # stdout

    def log_message(self, *args): pass

server = http.server.HTTPServer(('127.0.0.1', 3000), Handler)
server.handle_request()
")

CODE="${RESPONSE}"
echo "Got authorization code. Exchanging for token..."

# Exchange code for token (use --data-urlencode to handle special chars like == in secret)
TOKEN_JSON=$(curl -s -X POST "https://www.linkedin.com/oauth/v2/accessToken" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  --data-urlencode "grant_type=authorization_code" \
  --data-urlencode "code=${CODE}" \
  --data-urlencode "redirect_uri=${REDIRECT_URI}" \
  --data-urlencode "client_id=${CLIENT_ID}" \
  --data-urlencode "client_secret=${CLIENT_SECRET}")

echo "Token response: ${TOKEN_JSON}"

# Parse and save in the format linkedin-mcp expects
python3 -c "
import json, os, sys
from datetime import datetime, timezone, timedelta

resp = json.loads('''${TOKEN_JSON}''')
if 'access_token' not in resp:
    print('ERROR: No access_token in response', file=sys.stderr)
    print(json.dumps(resp, indent=2), file=sys.stderr)
    sys.exit(1)

expires_in = resp.get('expires_in', 5184000)
tokens = {
    'access_token': resp['access_token'],
    'refresh_token': resp.get('refresh_token'),
    'expires_at': (datetime.now(timezone.utc) + timedelta(seconds=expires_in)).isoformat(),
    'linkedin_id': None,
    'email': None,
}

config_dir = os.path.expanduser('~/.config/linkedin-mcp')
os.makedirs(config_dir, exist_ok=True)
path = os.path.join(config_dir, 'tokens.json')
with open(path, 'w') as f:
    json.dump(tokens, f, indent=2)
print(f'Token saved to {path}')
print(f'Expires: {tokens[\"expires_at\"]}')
"
