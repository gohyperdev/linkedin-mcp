# linkedin-mcp

A Model Context Protocol (MCP) server that lets AI agents publish posts to LinkedIn. Built in Rust for performance, using the official [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) (`rmcp`). Implements the LinkedIn "Share on LinkedIn" API with OAuth2 authentication, text posts, and image posts.

## Features

- **Text posts** — publish text-only posts to LinkedIn
- **Image posts** — upload a local image and publish it with text
- **Profile info** — retrieve the authenticated user's name, email, and LinkedIn ID
- **Auth status** — check token validity and expiration without making API calls
- **OAuth2 flow** — browser-based authorization with automatic token storage and refresh
- **stdio transport** — works with Claude Code, Claude Desktop, and any MCP-compatible client

## Prerequisites

- **Rust 1.75+** (install via [rustup](https://rustup.rs/))
- **LinkedIn Developer App** with the "Share on LinkedIn" product enabled
  - Create one at [LinkedIn Developer Portal](https://www.linkedin.com/developers/apps)
  - Add `http://localhost:3000/callback` as an authorized redirect URL
  - Enable the "Share on LinkedIn" product under the Products tab

## Setup

1. Clone the repository:

```bash
git clone https://github.com/gohyperdev/linkedin-mcp.git
cd linkedin-mcp
```

2. Build the project:

```bash
cargo build --release
```

3. Set environment variables (or export them in your shell):

```bash
export LINKEDIN_CLIENT_ID=your_client_id
export LINKEDIN_CLIENT_SECRET=your_client_secret
```

## Usage with Claude Code

```bash
claude mcp add linkedin-mcp -- \
  env LINKEDIN_CLIENT_ID=your_client_id \
  LINKEDIN_CLIENT_SECRET=your_client_secret \
  /path/to/linkedin-mcp/target/release/linkedin-mcp
```

## Usage with Claude Desktop

Add to your Claude Desktop MCP configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "linkedin": {
      "command": "/path/to/linkedin-mcp/target/release/linkedin-mcp",
      "env": {
        "LINKEDIN_CLIENT_ID": "your_client_id",
        "LINKEDIN_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

## MCP Tools

| Tool | Description |
|---|---|
| `get_profile` | Get LinkedIn profile info (name, email, ID, picture URL) |
| `create_text_post` | Create a text-only post. Params: `text`, optional `visibility` (PUBLIC/CONNECTIONS) |
| `share_article` | Publish an article (LinkedIn ARTICLE type) — share a URL with rich preview card. Params: `text`, `url`, optional `title`, `description`, `visibility` |
| `create_image_post` | Create a post with an image. Params: `text`, `image_path`, optional `image_alt`, `visibility` |
| `create_video_post` | Create a post with a video. Params: `text`, `video_path`, optional `title`, `description`, `visibility` |
| `auth_status` | Check authentication status, token expiry, associated email |

## Authentication

On first use of any tool that requires LinkedIn access, the server will:

1. Print an OAuth authorization URL to stderr
2. Wait for you to open the URL in a browser and authorize
3. Receive the callback on `http://localhost:3000/callback`
4. Store tokens in `~/.config/linkedin-mcp/tokens.json`

Subsequent uses will automatically refresh expired tokens. Access tokens are valid for 60 days, refresh tokens for 365 days.

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `LINKEDIN_CLIENT_ID` | Yes | OAuth2 client ID from LinkedIn Developer Portal |
| `LINKEDIN_CLIENT_SECRET` | Yes | OAuth2 client secret from LinkedIn Developer Portal |
| `RUST_LOG` | No | Logging level (e.g., `info`, `debug`). Logs go to stderr. |

## License

MIT -- see [LICENSE](LICENSE).

## Links

- [HyperDev GitHub](https://github.com/gohyperdev)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [LinkedIn Share API Docs](https://learn.microsoft.com/en-us/linkedin/consumer/integrations/self-serve/share-on-linkedin)
