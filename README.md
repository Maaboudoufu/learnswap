# LearnSwap

A peer skill exchange. Members list what they can teach and what they want to
learn, and LearnSwap only matches two people when each can teach the other
something. Both sides of a swap learn, since explaining a topic also deepens the
teacher's understanding (the protégé effect).

CMPE 131 course project. Full proposal: [docs/PROPOSAL.md](docs/PROPOSAL.md).

---

## Quick start

Requires [Rust](https://rustup.rs) 1.85+ (edition 2024). Node.js is not needed:
the Tailwind script downloads the standalone CLI on first run.

**Windows (PowerShell):**

```powershell
git clone <repo-url>
cd cmpe131_project
.\scripts\tailwind.ps1     # builds assets/css/app.css
cargo run
```

**macOS / Linux:**

```bash
git clone <repo-url>
cd cmpe131_project
./scripts/tailwind.sh      # builds assets/css/app.css
cargo run
```

Open <http://localhost:3000>. The first run creates `learnswap.db` and seeds five
sample members. Log in as `ana`, `marcus`, `priya`, `tom` or `lena` (all
`@example.edu`) with password `learnswap`, which is also printed in the startup log.

If port 3000 is taken: `PORT=4000 cargo run` (PowerShell: `$env:PORT=4000; cargo run`).

### Database

`DATABASE_URL` picks the backend at runtime and defaults to a local SQLite file:

```bash
# SQLite (default) — a file in the repo root, git-ignored
DATABASE_URL="sqlite://learnswap.db?mode=rwc" cargo run

# Postgres — same code, same schema
DATABASE_URL="postgres://user:pass@localhost/learnswap" cargo run
```

Migrations in `migrations/` run on startup. Seeding only runs on an empty table,
so restarts never duplicate or overwrite accounts. To reset, delete
`learnswap.db`.

### Development

Run these in two terminals:

| Terminal | Windows | macOS / Linux |
| --- | --- | --- |
| Styles | `.\scripts\tailwind.ps1 watch` | `./scripts/tailwind.sh watch` |
| Server | `cargo run` | `cargo run` |

Templates reload from disk, so edits in `templates/` show up on refresh. Only
`.rs` changes need a restart. Start `cargo run` from the repo root, since
templates and assets load by relative path (the server warns if it can't find
them).

---

## Stack

| Piece | Role |
| --- | --- |
| [Axum](https://docs.rs/axum) | HTTP server and routing. |
| [MiniJinja](https://docs.rs/minijinja) | Jinja-syntax HTML templates, so front-end work is HTML, not Rust. |
| [HTMX](https://htmx.org) | Server sends HTML fragments; HTMX swaps them into the page. |
| [Tailwind](https://tailwindcss.com) (standalone CLI) | Utility-class styling. |
| [sqlx](https://docs.rs/sqlx) | SQLite or Postgres, chosen from `DATABASE_URL`. |
| [tower-sessions](https://docs.rs/tower-sessions) + [argon2](https://docs.rs/argon2) | Cookie sessions and password hashing. |

HTMX means no JavaScript bundle, no front-end build, and no client state to sync.
Every HTMX interaction also works without JavaScript: handlers check the
`HX-Request` header and return either a fragment or a full page. Keep it that way
when adding features, so any URL can be opened directly in a browser for
debugging.

---

## Project layout

```
src/
  main.rs        Startup: logging, port, bind, serve.
  lib.rs         Module list. Tests import the app through here.
  models.rs      User, Skill, Swap — including the two-way matching rule.
  store.rs       Every SQL query. The ONLY module that knows about storage.
  db.rs          Pool setup, migrations, and the one SQLite/Postgres difference.
  auth.rs        Argon2 hashing and the "who is logged in?" extractor.
  state.rs       Shared state handed to every handler.
  templates.rs   MiniJinja setup and the render() helper.
  htmx.rs        HxRequest extractor: "did this come from HTMX?"
  error.rs       AppError and how it turns into a response.
  routes/
    mod.rs       The route table. Start here to find a handler.
    pages.rs     Landing page, health check, 404.
    members.rs   Browse, search, profile.
    auth.rs      Register, log in, log out.

templates/
  layouts/base.html    Page shell: nav, footer, <head>.
  pages/               Full pages. Each extends the layout.
  partials/            Fragments HTMX swaps in. Also included by pages.
  macros.html          Reusable bits (skill pills, member cards).

migrations/            SQL schema, applied automatically on startup.
assets/css/input.css   Tailwind source. app.css is generated — do not edit it.
scripts/               Tailwind build scripts.
tests/routes.rs        End-to-end tests against the real router.
```

### Adding a page

1. Add a handler in `src/routes/` and register it in `src/routes/mod.rs`.
2. Add a template in `templates/pages/` starting with `{% extends "layouts/base.html" %}`.
3. Add a test in `tests/routes.rs` (copy an existing one).

For UI that updates in place, put a template in `templates/partials/`, render it
from a handler, and point `hx-get` or `hx-post` at that route. See
`templates/pages/browse.html` for the smallest example.

---

## Tests

```bash
cargo test
```

`tests/routes.rs` sends real requests through the real router in-process, each
test with its own in-memory SQLite database. Coverage: routing, templates,
search, matching, form validation, registration, login, and HTMX vs. plain
requests.

Against Postgres (tests share one database, so run single-threaded):

```bash
TEST_DATABASE_URL="postgres://user:pass@localhost/learnswap_test" cargo test --test routes -- --test-threads=1
```

Before pushing, run what CI runs:

```bash
cargo fmt --all
cargo clippy --all-targets -- --deny warnings
cargo test
```

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs on every push to
`main` and every PR. All jobs must pass before merging:

- **rust**: format check, Clippy (warnings denied), tests, release build.
- **postgres**: the test suite against a Postgres 16 container.
- **docker**: builds the image, runs it, and checks it serves a styled page.
- **css**: builds the stylesheet and fails if it's suspiciously small, which
  means the `@source` globs in `assets/css/input.css` stopped matching templates.

## Generated files

`assets/css/app.css`, `.tailwind/` and `learnswap.db` are git-ignored. Committing
`app.css` would cause merge conflicts whenever two people touch a template. If
styles look wrong after pulling, rerun the Tailwind script.

---

## Deploying with Docker

The image compiles everything internally, so it builds on both x86-64 and Oracle
Cloud Ampere (ARM). From the cloned repo on the server:

```bash
git pull
docker build -t learnswap .
docker run -d --name learnswap --restart unless-stopped -p 80:3000 -v learnswap-data:/data learnswap
```

This serves on port 80 and keeps SQLite in the `learnswap-data` volume across
rebuilds. To update, `git pull && docker build -t learnswap . && docker rm -f
learnswap`, then run again.

For Postgres:

```bash
docker run -d --name learnswap --restart unless-stopped -p 80:3000 -e DATABASE_URL="postgres://user:pass@db-host/learnswap" learnswap
```

The image runs as a non-root user and contains only the binary, templates and
compiled CSS.

### Image settings

| Variable | Default in image | Notes |
| --- | --- | --- |
| `HOST` | `0.0.0.0` | Must be `0.0.0.0` in a container or published ports won't reach the app. Local default is `127.0.0.1`. |
| `PORT` | `3000` | Port inside the container; map it with `-p`. |
| `DATABASE_URL` | `sqlite:///data/learnswap.db?mode=rwc` | `/data` is the volume. |
| `RUST_LOG` | `learnswap=info,tower_http=info,warn` | Set to `debug` when troubleshooting. |

### Oracle Cloud gotchas

1. **Open the port in two places**: an ingress rule in the subnet's Security List
   (or NSG) in the OCI console, and the instance's own firewall, which drops
   traffic by default on Oracle's stock images:
   ```bash
   sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
   sudo netfilter-persistent save        # Ubuntu images
   # Oracle Linux images use firewalld instead:
   # sudo firewall-cmd --permanent --add-port=80/tcp && sudo firewall-cmd --reload
   ```
2. **Builds need memory.** Fine on an Ampere A1, but a 1 GB `E2.1.Micro` can
   OOM-kill the build with no error message. Add swap or use a bigger shape.

---

## Not built yet

- **Profile editing.** Skills can't be changed after registering.
- **Password reset.** No email is sent, so a lost password means a new account.
- **Persistent sessions.** Sessions are in memory: a restart logs everyone out,
  and multiple instances don't share them. `tower-sessions-sqlx-store` can reuse
  the existing pool when needed.
- **Contacting a match.** No messaging or scheduling.
- **Ranking.** Matches sort by number of skills in play only, not location,
  availability or past swaps.
