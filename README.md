# LearnSwap

A peer skill exchange. Members list what they can teach and what they want to
learn, and LearnSwap only matches two people when each can teach the other
something. Both sides of a swap learn, since explaining a topic also deepens the
teacher's understanding (the protégé effect).

CMPE 131 course project.

---

## Quick start

Requires [Rust](https://rustup.rs) 1.85+ (edition 2024).

```bash
git clone <repo-url>
cd learnswap
cargo run
```

Open <http://localhost:3000>. The server binds `0.0.0.0:3000` (hard-coded in
`src/main.rs`).

| Route | Returns |
| --- | --- |
| `GET /` | Homepage, rendered from `templates/index.html`. |
| `GET /health` | Plain-text message if the server is up. |

---

## Stack

| Piece | Role |
| --- | --- |
| [Axum](https://docs.rs/axum) | HTTP server and routing. |
| [Tokio](https://tokio.rs) | Async runtime. |
| [MiniJinja](https://docs.rs/minijinja) | Jinja-syntax HTML templates. |
| [Tailwind](https://tailwindcss.com) (browser CDN build) | Styling, loaded by a `<script>` tag in the template. No build step. |

---

## Project layout

```
src/
  main.rs          Startup: template setup, route table, bind, serve.
  routes/
    mod.rs         Module list for the handlers.
    pages.rs       GET /  renders index.html.
    health.rs      GET /health

templates/
  index.html       Homepage (nav, hero, Tailwind via CDN).

static/images/     Images used by templates.
Dockerfile         Multi-stage build: rust builder, debian-slim runtime.
.github/workflows/ CI (see below).
```

Templates are embedded into the binary with `include_str!`, so template edits
need a restart (`cargo run` again).

### Adding a page

1. Add the template in `templates/`.
2. Register it in `src/main.rs` with `env.add_template("name.html", include_str!(...))`.
3. Add a handler in `src/routes/` (copy `pages.rs`) and export it in `src/routes/mod.rs`.
4. Add a `.route(...)` for it in `src/main.rs`.

---

## CI

Workflows in `.github/workflows/` run on pushes and PRs to `main`:

- `rust.yml` runs `cargo build` and `cargo test`.
- `docker-publish.yml` builds the image for `linux/amd64` and `linux/arm64`
  and publishes it to GHCR (also runs nightly).
- `codeql.yml` runs code scanning.
- `dependency-review.yml` flags vulnerable dependencies in PRs.

Before pushing:

```bash
cargo fmt --all
cargo clippy --all-targets
cargo build
```

---

## Docker

```bash
docker build -t learnswap .
docker run -d --name learnswap --restart unless-stopped -p 80:3000 learnswap
```

The image builds on both x86-64 and ARM (e.g. Oracle Cloud Ampere). To update:
`git pull && docker build -t learnswap . && docker rm -f learnswap`, then run
again.

### Oracle Cloud gotchas

1. Open the port in two places: an ingress rule in the subnet's Security List
   (or NSG) in the OCI console, and the instance's own firewall, which drops
   traffic by default on Oracle's stock images:
   ```bash
   sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
   sudo netfilter-persistent save        # Ubuntu images
   # Oracle Linux images use firewalld instead:
   # sudo firewall-cmd --permanent --add-port=80/tcp && sudo firewall-cmd --reload
   ```
2. The build needs memory. An Ampere A1 handles it, but a 1 GB `E2.1.Micro` can
   OOM-kill the build with no error message. Add swap or use a bigger shape.

---

## Known issues

- The hero image doesn't load. `templates/index.html` points at
  `/static/images/learnswap-hero.png`, but no route serves `static/`, and the
  Dockerfile doesn't copy it into the image.
- The port is hard-coded to 3000. There is no `PORT` env var yet.

## Not built yet

- Accounts: register, log in, log out.
- Member profiles with teach / learn skill lists.
- Browse and search members.
- Two-way matching (both people can teach each other something).
- Database storage.
- Tests.
