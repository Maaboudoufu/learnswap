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

Then open <http://localhost:3000>. The app starts with five sample members so
there is something to click on immediately.

Set `PORT` if 3000 is taken: `PORT=4000 cargo run` (PowerShell:
`$env:PORT=4000; cargo run`).

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
  models.rs      Member, Skill, Swap — including the two-way matching rule.
  store.rs       Where members live. The ONLY module that knows about storage.
  state.rs       Shared state handed to every handler.
  templates.rs   MiniJinja setup and the render() helper.
  htmx.rs        HxRequest extractor: "did this come from HTMX?"
  error.rs       AppError and how it turns into a response.
  routes/
    mod.rs       The route table. Start here to find a handler.
    pages.rs     Landing page, health check, 404.
    members.rs   Browse, search, profile, join.

templates/
  layouts/base.html    Page shell: nav, footer, <head>.
  pages/               Full pages. Each extends the layout.
  partials/            Fragments HTMX swaps in. Also included by pages.
  macros.html          Reusable bits (skill pills, member cards).

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
validation and the HTMX-vs-plain-request split.

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
- **css** — builds the stylesheet and fails if it comes out suspiciously small,
  which is what happens when the `@source` globs in `assets/css/input.css` stop
  matching the templates.

Both must pass before a pull request is merged.

## Generated files

`assets/css/app.css` and `.tailwind/` are git-ignored. `app.css` is built from
`assets/css/input.css`; committing it would produce a merge conflict in
machine-generated output every time two people touch a template. Run the
Tailwind script after pulling if styles look wrong.

---

## Not decided yet

These are open and deliberately not built:

- **Persistence.** Members are held in memory and disappear on restart. All
  storage goes through `Store` in `src/store.rs`, so adding SQLite (via `sqlx`)
  means reimplementing that one type — no handler changes.
- **Accounts and authentication.** Anyone can create a profile; there are no
  logins and no way to edit or delete one.
- **Contacting a match.** Profiles show who you could swap with, but there is no
  messaging or scheduling yet.
- **Ranking.** Matches are sorted by how many skills are in play. Location,
  availability and past swaps are not considered.
