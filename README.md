# linkedin-mcp

A Model Context Protocol (MCP) server that lets AI agents publish posts to LinkedIn. Built in Rust for performance, using the official [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) (`rmcp`).

## Features

- **Text posts** — publish text-only posts (up to 3000 chars)
- **URL/link sharing** — share a URL with a rich preview card (thumbnail, title, description)
- **Image posts** — upload a local image and publish it with text
- **Video posts** — upload a local video and publish it with text
- **Profile info** — retrieve the authenticated user's name, email, and LinkedIn ID
- **Auth status** — check token validity and expiration
- **Automatic token refresh** — access tokens (60 days) and refresh tokens (365 days)
- **stdio transport** — works with Claude Code, Claude Desktop, and any MCP-compatible client

## Installation

### Option A: Download pre-built binary (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/gohyperdev/linkedin-mcp/releases):

| Platform | Binary |
|---|---|
| macOS Apple Silicon | `linkedin-mcp-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `linkedin-mcp-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `linkedin-mcp-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `linkedin-mcp-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# Example for macOS Apple Silicon:
curl -L https://github.com/gohyperdev/linkedin-mcp/releases/latest/download/linkedin-mcp-aarch64-apple-darwin.tar.gz | tar xz
chmod +x linkedin-mcp
sudo mv linkedin-mcp /usr/local/bin/
```

### Option B: Build from source

Requires [Rust 1.75+](https://rustup.rs/).

```bash
git clone https://github.com/gohyperdev/linkedin-mcp.git
cd linkedin-mcp
cargo build --release
# Binary at: target/release/linkedin-mcp
```

## LinkedIn App Setup

Before using this MCP server, you need a LinkedIn Developer App:

1. Go to [LinkedIn Developer Portal](https://www.linkedin.com/developers/apps) and click **Create app**
2. Fill in the app details (name, logo, company page)
3. In the **Products** tab, add:
   - **Share on LinkedIn** (grants `w_member_social` scope — required for posting)
   - **Sign In with LinkedIn using OpenID Connect** (grants `openid profile email` scopes — required for profile info)
4. In the **Auth** tab:
   - Copy your **Client ID** and **Client Secret**
   - Add `http://localhost:3000/callback` under **Authorized redirect URLs**

## Security

**Your Client ID and Client Secret are sensitive credentials. Never commit them to git.**

- Pass credentials via **environment variables** only: `LINKEDIN_CLIENT_ID` and `LINKEDIN_CLIENT_SECRET`
- The `.env` file is in `.gitignore` — use it for local development
- OAuth tokens are stored locally at `~/.config/linkedin-mcp/tokens.json` — this file contains your access token, treat it as a secret
- The MCP server reads credentials from environment variables at runtime, never from source code

## Authentication (One-Time Setup)

Before the MCP tools can post to LinkedIn, you need to authorize the app once. Run the included setup script:

```bash
LINKEDIN_CLIENT_ID=your_client_id \
LINKEDIN_CLIENT_SECRET=your_client_secret \
./auth-cli.sh
```

This will:

1. Print an OAuth authorization URL — open it in your browser
2. You'll see LinkedIn's consent screen — click **Allow**
3. LinkedIn redirects to `localhost:3000/callback` — the script catches it
4. Access token is saved to `~/.config/linkedin-mcp/tokens.json`

**That's it.** The token is valid for 60 days and auto-refreshes (refresh token valid 365 days). You only need to re-run `auth-cli.sh` if both tokens expire.

## Usage with Claude Code

```bash
claude mcp add linkedin-mcp -s user -- \
  env LINKEDIN_CLIENT_ID=your_client_id \
  LINKEDIN_CLIENT_SECRET=your_client_secret \
  /usr/local/bin/linkedin-mcp
```

Or for a specific project only (`-s project`):

```bash
claude mcp add linkedin-mcp -s project -- \
  env LINKEDIN_CLIENT_ID=your_client_id \
  LINKEDIN_CLIENT_SECRET=your_client_secret \
  /path/to/linkedin-mcp
```

## Usage with Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "linkedin": {
      "command": "/usr/local/bin/linkedin-mcp",
      "env": {
        "LINKEDIN_CLIENT_ID": "your_client_id",
        "LINKEDIN_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

Restart Claude Desktop after saving.

## MCP Tools

| Tool | Description | Key Params |
|---|---|---|
| `get_profile` | Get LinkedIn profile info | — |
| `create_text_post` | Publish a text-only post | `text`, `visibility?` |
| `share_article` | Share a URL with preview card | `text`, `url`, `title?`, `description?`, `visibility?` |
| `create_image_post` | Publish a post with an image | `text`, `image_path`, `image_alt?`, `visibility?` |
| `create_video_post` | Publish a post with a video | `text`, `video_path`, `title?`, `description?`, `visibility?` |
| `auth_status` | Check authentication status | — |

**Visibility** options: `PUBLIC` (default) or `CONNECTIONS` (1st-degree only).

## Limitations

- **Personal profile only.** Company Page posting requires the "Community Management API" product which must be the only product on the app (separate LinkedIn app needed).
- **No native LinkedIn articles.** The LinkedIn API does not support creating long-form articles (the ones at `linkedin.com/pulse/...` with rich-text formatting). The `share_article` tool shares a URL with a preview card — it's a regular feed post, not a native article.
- **No LinkedIn Newsletter editions.** Newsletter content can only be created through the LinkedIn web UI.
- **No draft posts.** The "Share on LinkedIn" API only supports `lifecycleState: PUBLISHED`. Draft support requires the Community Management API.
- **Rate limits:** 150 requests/day per member, 100,000/day per application.

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `LINKEDIN_CLIENT_ID` | Yes | OAuth2 client ID from LinkedIn Developer Portal |
| `LINKEDIN_CLIENT_SECRET` | Yes | OAuth2 client secret from LinkedIn Developer Portal |
| `RUST_LOG` | No | Logging level (`info`, `debug`, `trace`). Logs go to stderr. |

## Token Storage

Tokens are stored at `~/.config/linkedin-mcp/tokens.json`:

```json
{
  "access_token": "AQV...",
  "refresh_token": null,
  "expires_at": "2026-06-05T01:06:11Z",
  "linkedin_id": "sIhlYJw_ja",
  "email": "user@example.com"
}
```

To revoke access, delete this file and remove the app from [LinkedIn Settings > Permitted services](https://www.linkedin.com/mypreferences/d/manage-third-party-apps).

## License

MIT — see [LICENSE](LICENSE).

## Links

- [HyperDev GitHub](https://github.com/gohyperdev)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [LinkedIn Share API Docs](https://learn.microsoft.com/en-us/linkedin/consumer/integrations/self-serve/share-on-linkedin)
- [LinkedIn Developer Portal](https://www.linkedin.com/developers/apps)
