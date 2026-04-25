# zammad-cli

[Zammad](https://zammad.org) helpdesk için single-binary Rust CLI. 13 operasyon, nested subcommand yapısı (ticket search, ticket update, ticket article add, org list, user search, system states, vb.).

## Stack

- **Dil**: Rust 2021
- **Build**: `cargo` (rustc 1.94+)
- **Bağımlılıklar**:
  - `clap` 4.6 (derive) — argparse + nested subcommand
  - `reqwest` 0.13 (rustls + json + query) — HTTP client
  - `tokio` 1.52 (rt-multi-thread, macros) — async runtime
  - `serde` + `serde_json` — JSON
  - `comfy-table` 7.2 — tables
  - `colored` 3.1 — terminal colors
  - `futures` 0.3 — `try_join_all` for parallel `ticket overview`
  - `anyhow` — error wrapping

## Dil

Türkçe iletişim, İngilizce kod yorumu + commit mesajı.

## Komutlar

```bash
cargo build                                # debug
cargo build --release                      # release (~3.1 MB binary)
cargo run -- ticket overview               # local çalıştır (env gerekli)
cargo clippy --all-targets -- -D warnings  # lint
cargo fmt --all                            # format
cargo test                                 # test
```

Binary kullanımı:

```bash
export ZAMMAD_URL=https://support.example.com
export ZAMMAD_TOKEN=your-api-token

zammad-cli ticket list --state open
zammad-cli ticket get #61234
zammad-cli ticket get 42
zammad-cli ticket article add #61234 --body "Investigating"
zammad-cli ticket overview
zammad-cli --json org list | jq '.[] | .name'
```

## Proje Yapısı

```
src/
├── main.rs                 clap parser + tokio runtime, dispatch
├── config.rs               env var (ZAMMAD_URL/_TOKEN) reader
├── client.rs               reqwest wrapper (Token token=... auth, error format)
├── types.rs                serde structs (Ticket, Article, Organization, User, NamedItem)
├── output.rs               render() dispatch + table/detail printers
├── util.rs                 truncate, build_search_query (auto-quote), insert_opt_str
└── commands/
    ├── ticket.rs           search/list/get/articles/create/update/article add/overview
    ├── org.rs              list/search
    ├── user.rs             search
    └── system.rs           groups/states/priorities

skills/zammad-cli/SKILL.md  Claude Code skill (workflow wrapper)
.github/workflows/          CI (rustfmt + clippy + test) + Release (multi-target)
```

## Kod Konvansiyonları

- `cargo fmt --all` ile formatla
- `cargo clippy --all-targets -- -D warnings` temiz olmalı
- `anyhow::Result` + `with_context` ile hata zinciri
- HTTP body construction: `util::insert_opt_str` helper kullan, manual Map.insert tekrarı yok
- Output dispatch: `output::render(&value, json, |v| print_human(v))`
- Search query: `util::build_search_query(&parts)` — `value.contains(' ')` ise otomatik quote
- Ticket ID semantiği: `#61234` (ticket number) → API'da search; `42` (plain int) → internal ID. `resolve_ticket_id` çevirir.

## API Notları

- Auth header format: `Authorization: Token token=<TOKEN>` (Zammad-spesifik, Bearer DEĞİL)
- Search query syntax: `field.subfield:value AND field.subfield:"value with space"`
  - `state.name:open`
  - `group.name:"Destek Ekibi"`
  - `priority.name:"3 high"`
  - `owner.email:agent@example.com`
- Ticket states: `new`, `open`, `closed`, `pending reminder`, `pending close`
- Ticket priorities: `1 low`, `2 normal`, `3 high`
- Article `internal: true` (default) = dahili not, `false` = halka açık yanıt
- `ticket overview` 5 state için paralel `try_join_all` ile fetch yapar (per_page=100, .len() ile sayar — büyük instance'larda undercount riski var)

## Skill

`skills/zammad-cli/SKILL.md` Claude Code skill'i tetik:
- `/zammad-cli`, "ticket ara", "ticket aç", "ticket kapat", "müşteri tickets"
- `--json` ile çağırıp parse eder
- AskUserQuestion ile destructive action onayı alır

## Release

Tag push → GitHub Actions multi-target build (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64) + GitHub Release.

```bash
git tag v0.1.0
git push origin v0.1.0
```
