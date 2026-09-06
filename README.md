# LearnSwap

A peer skill exchange platform. Members list what they can teach and what they
want to learn, and LearnSwap surfaces the pairs where the trade works **in both
directions** — you only appear as someone's match if they can teach you
something too.

That two-way rule is the whole point. Course marketplaces sell one-way
instruction; LearnSwap is built around the protégé effect, where explaining a
topic deepens the teacher's own understanding, so both sides of a swap are
learning.

CMPE 131 course project. The full proposal is in [docs/PROPOSAL.md](docs/PROPOSAL.md).

---

## Quick start

You need [Rust](https://rustup.rs) (1.85 or newer, for edition 2024). You do
**not** need Node.js — the Tailwind build uses the standalone CLI, a single
binary the script downloads for you on first run.

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

Then open <http://localhost:3000>. On first run it creates `learnswap.db` and
seeds five sample members, so there is something to click on immediately.

**Log in as any of them** with their `@example.edu` address and the password
`learnswap` (printed in the startup log): `ana@example.edu`, `marcus@example.edu`,
`priya@example.edu`, `tom@example.edu`, `lena@example.edu`.

Set `PORT` if 3000 is taken: `PORT=4000 cargo run` (PowerShell:
`$env:PORT=4000; cargo run`).

### Database

`DATABASE_URL` picks the backend at runtime. Unset, it defaults to a local
SQLite file, which is what you want for coursework:

```bash
# SQLite (default) — a file in the repo root, git-ignored
DATABASE_URL="sqlite://learnswap.db?mode=rwc" cargo run

# Postgres — same code, same schema
DATABASE_URL="postgres://user:pass@localhost/learnswap" cargo run
```

Migrations in `migrations/` run automatically on startup, so there is no
separate setup step. To start over, delete `learnswap.db` and run again.

Seeding only happens when the table is empty, so restarting never duplicates
members and never overwrites an account you created.

### While you are working

Run these in two terminals and neither needs restarting as you edit:

| Terminal | Windows | macOS / Linux |
| --- | --- | --- |
| Styles | `.\scripts\tailwind.ps1 watch` | `./scripts/tailwind.sh watch` |
| Server | `cargo run` | `cargo run` |

Templates are watched and reloaded from disk, so **editing anything in
`templates/` shows up on the next page refresh** — no rebuild, no restart. Only
changes to `.rs` files need `cargo run` again.

> `cargo run` must be started from the repository root. Templates and static
> assets are loaded by relative path; the server logs a warning if it cannot
> find them.

---

## The stack, and why

| Piece | Role |
| --- | --- |
| **Rust + [Axum](https://docs.rs/axum)** | HTTP server, routing, request handling. |
| **[MiniJinja](https://docs.rs/minijinja)** | HTML templates with Jinja syntax. Whoever is doing front-end work writes HTML, not Rust. |
| **[HTMX](https://htmx.org)** | Server-side rendering with live updates. The server sends finished HTML and HTMX swaps it into the page. |
| **[Tailwind](https://tailwindcss.com) (standalone CLI)** | Styling via utility classes in the markup. |
| **[sqlx](https://docs.rs/sqlx)** | Accounts in SQLite or Postgres, picked at runtime from `DATABASE_URL`. |
| **[tower-sessions](https://docs.rs/tower-sessions) + [argon2](https://docs.rs/argon2)** | Cookie sessions and password hashing. |

The HTMX choice is what keeps the front-end simple: there is no JavaScript
bundle, no build step for the app itself, and no client-side state to keep in
sync with the server. Search results and form submissions come back as HTML
fragments that get dropped into place.

Every HTMX-powered interaction also works with JavaScript disabled — the search
box has a real submit button, and the join form is a real `<form>` POST that
redirects. Handlers check for the `HX-Request` header and return a fragment or a
full page accordingly. Keep that property when you add features; it makes
debugging much easier, because you can hit any URL directly in a browser.

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

1. Add a handler in `src/routes/`.
2. Register it in `src/routes/mod.rs`.
3. Add a template in `templates/pages/` that starts with
   `{% extends "layouts/base.html" %}`.
4. Add a test in `tests/routes.rs` — copy an existing one, they are short.

For a piece of UI that updates in place, make it a template in
`templates/partials/`, render it from a handler, and point `hx-get` or `hx-post`
at that route. `templates/pages/browse.html` is the smallest complete example.

---

## Tests

```bash
cargo test
```

The tests in `tests/routes.rs` build the real router and send real requests
through it in-process — no server to start, no browser, runs in milliseconds.
They cover routing, template rendering, search, the matching rule, form
validation, registration, login, and the HTMX-vs-plain-request split. Each test
gets its own migrated in-memory SQLite database.

To run them against Postgres instead — the tests share one database, so they
cannot run in parallel:

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
`main` and every pull request:

- **rust** — formatting check, Clippy with warnings denied, tests, release build.
- **postgres** — runs the same test suite against a real Postgres 16 service
  container. The SQLite path is covered by the `rust` job; this one proves the
  Postgres half of the `Any` driver actually works.
- **docker** — builds the deployment image, starts the container and checks it
  serves a real, styled page. A broken Dockerfile fails here, not on the server.
- **css** — builds the stylesheet and fails if it comes out suspiciously small,
  which is what happens when the `@source` globs in `assets/css/input.css` stop
  matching the templates.

Both must pass before a pull request is merged.

## Generated files

`assets/css/app.css`, `.tailwind/` and `learnswap.db` are git-ignored. `app.css` is built from
`assets/css/input.css`; committing it would produce a merge conflict in
machine-generated output every time two people touch a template. Run the
Tailwind script after pulling if styles look wrong.

---

## Deploying with Docker

The image compiles everything inside itself, so it builds unchanged on x86-64
and on the ARM cores of an Oracle Cloud Ampere instance. On the server, from the
cloned repo:

```bash
git pull
docker build -t learnswap .
docker run -d --name learnswap --restart unless-stopped -p 80:3000 -v learnswap-data:/data learnswap
```

That serves on port 80 and keeps the SQLite database in the `learnswap-data`
volume, so it survives rebuilds. To update: `git pull && docker build -t
learnswap . && docker rm -f learnswap` then run again.

To use Postgres instead, override one variable:

```bash
docker run -d --name learnswap --restart unless-stopped -p 80:3000 -e DATABASE_URL="postgres://user:pass@db-host/learnswap" learnswap
```

### Settings the image controls

| Variable | Default in image | Notes |
| --- | --- | --- |
| `HOST` | `0.0.0.0` | Must stay `0.0.0.0` in a container or published ports never reach the app. Locally it defaults to `127.0.0.1`. |
| `PORT` | `3000` | The port *inside* the container; map it with `-p`. |
| `DATABASE_URL` | `sqlite:///data/learnswap.db?mode=rwc` | `/data` is the volume. |
| `RUST_LOG` | `learnswap=info,tower_http=info,warn` | Raise to `debug` when something misbehaves. |

### Two Oracle Cloud gotchas

1. **Opening the port takes two steps.** Add an ingress rule to the subnet's
   Security List (or Network Security Group) in the OCI console, *and* open it on
   the instance itself — Oracle's stock images ship with restrictive local
   firewall rules that silently drop traffic even after the console rule exists:
   ```bash
   sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
   sudo netfilter-persistent save        # Ubuntu images
   # Oracle Linux images use firewalld instead:
   # sudo firewall-cmd --permanent --add-port=80/tcp && sudo firewall-cmd --reload
   ```
2. **Give the build enough memory.** Compiling from scratch is memory-hungry;
   it is comfortable on an Ampere A1 instance but can get killed by the OOM
   reaper on a 1 GB `E2.1.Micro`. If the build dies without an error message,
   that is why — add swap or build on a bigger shape.

The image runs as a non-root user and carries no compiler or build tooling: only
the binary, the templates and the compiled CSS.

---

## Not decided yet

These are open and deliberately not built:

- **Editing your profile.** You can register and log in, but not change your
  skills afterwards — you would have to register again.
- **Password reset.** No email is sent anywhere, so a forgotten password means a
  new account.
- **Session storage.** Sessions live in memory, so restarting the server signs
  everyone out and a second instance would not share them. `tower-sessions-sqlx-store`
  can use the pool we already have when that matters.
- **Contacting a match.** Profiles show who you could swap with, but there is no
  messaging or scheduling yet.
- **Ranking.** Matches are sorted by how many skills are in play. Location,
  availability and past swaps are not considered.
