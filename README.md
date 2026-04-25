# zammad-cli

[![CI](https://github.com/omert11/zammad-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/omert11/zammad-cli/actions/workflows/ci.yml)
[![Release](https://github.com/omert11/zammad-cli/actions/workflows/release.yml/badge.svg)](https://github.com/omert11/zammad-cli/actions/workflows/release.yml)

Single-binary Rust CLI for [Zammad](https://zammad.org) helpdesk — search/manage tickets, organizations, users, articles from the terminal.

## Features

- **13 operations** as nested subcommands (`ticket search`, `ticket update`, `ticket article add`, `org list`, `user search`, `system states`, …)
- **Pretty colored tables** by default, `--json` for piping
- **`#NUMBER` vs internal ID** — `ticket get #61234` searches by ticket number, `ticket get 42` uses internal ID
- **Auto-quoted search filters** — values with spaces (e.g. `"pending reminder"`, `"Destek Ekibi"`) are quoted automatically
- **Parallel `ticket overview`** — 5 state counts fetched concurrently
- **Single static binary** (~3 MB, no runtime)

## Install

### Prebuilt binaries (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/omert11/zammad-cli/releases/latest):

| Platform | Archive |
|----------|---------|
| Linux x86_64 | `zammad-cli-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `zammad-cli-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 (Intel) | `zammad-cli-x86_64-apple-darwin.tar.gz` |
| macOS aarch64 (Apple Silicon) | `zammad-cli-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `zammad-cli-x86_64-pc-windows-msvc.zip` |

Quick install (Linux/macOS):

```bash
TARGET=$(rustc -vV 2>/dev/null | sed -n 's/host: //p')
[ -z "$TARGET" ] && TARGET=$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)
curl -L "https://github.com/omert11/zammad-cli/releases/latest/download/zammad-cli-${TARGET}.tar.gz" \
  | tar xz -C /tmp \
  && sudo mv /tmp/zammad-cli /usr/local/bin/zammad-cli \
  && zammad-cli --version
```

### From source

```bash
cargo install --git https://github.com/omert11/zammad-cli
```

### Build locally

```bash
git clone https://github.com/omert11/zammad-cli
cd zammad-cli
cargo build --release
# binary: ./target/release/zammad-cli
```

## Configuration

```bash
export ZAMMAD_URL=https://support.example.com
export ZAMMAD_TOKEN=your-api-token
```

Generate a token from Zammad → Profile → Token Access (`ticket.agent` + `admin` permissions).

## Usage

### Tickets

```bash
zammad-cli ticket search "performance issue"
zammad-cli ticket search "state.name:open AND priority.name:\"3 high\"" --limit 50

zammad-cli ticket list --state open
zammad-cli ticket list --state new --priority "3 high"
zammad-cli ticket list --customer info@example.com --state open
zammad-cli ticket list --group "Destek Ekibi"

zammad-cli ticket get #61234            # by ticket number
zammad-cli ticket get 42                # by internal ID
zammad-cli ticket articles #61234

zammad-cli ticket create \
  --title "Performance regression" \
  --body "Saw 3s page load on /dashboard" \
  --group "Destek Ekibi" \
  --customer info@example.com \
  --priority "3 high" \
  --state new

zammad-cli ticket update #61234 --state closed
zammad-cli ticket update #61234 --priority "3 high" --owner agent@example.com
zammad-cli ticket update #61234 --title "Updated title"

zammad-cli ticket article add #61234 --body "Investigating now"          # internal note (default)
zammad-cli ticket article add #61234 --body "Reply to customer" --public  # public reply
zammad-cli ticket article add #61234 --body "Note" --subject "Re: Issue"

zammad-cli ticket overview                # ticket counts by state (parallel fetch)
```

### Organizations

```bash
zammad-cli org list
zammad-cli org search "diji"
```

### Users

```bash
zammad-cli user search "john@example.com"
zammad-cli user search "0532"             # phone
zammad-cli user search "John Doe"
```

### System

```bash
zammad-cli system groups
zammad-cli system states
zammad-cli system priorities
```

## Search Syntax

`ticket list` builds a Zammad search query from named filters:

| Flag | Field |
|------|-------|
| `--state` | `state.name` |
| `--group` | `group.name` (auto-quoted if value has spaces) |
| `--owner` | `owner.email` |
| `--organization` | `organization.name` |
| `--customer` | `customer.email` |
| `--priority` | `priority.name` (auto-quoted if value has spaces) |

Combined with `AND`. For ad-hoc queries use `ticket search "<raw zammad query>"`.

## Output

Default: pretty colored tables. `--json` (global flag): JSON to stdout.

```bash
zammad-cli --json ticket list --state open | jq 'map(.title)'
zammad-cli --json ticket overview          # {state: count, ...}
```

## Dependencies

- [`clap`](https://crates.io/crates/clap) — argparse with derive macros
- [`reqwest`](https://crates.io/crates/reqwest) — HTTP client (rustls TLS, no OpenSSL)
- [`tokio`](https://crates.io/crates/tokio) — async runtime
- [`serde` / `serde_json`](https://crates.io/crates/serde) — JSON I/O
- [`anyhow`](https://crates.io/crates/anyhow) — error handling
- [`comfy-table`](https://crates.io/crates/comfy-table) — terminal tables
- [`colored`](https://crates.io/crates/colored) — terminal colors
- [`futures`](https://crates.io/crates/futures) — `try_join_all` for parallel `ticket overview`

## License

MIT
