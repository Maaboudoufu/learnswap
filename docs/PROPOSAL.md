# Proposal for web app: LearnSwap

## Business Domain

This project falls under the tutoring and skill-sharing domain. It focuses on
connecting individuals who want to learn new skills with others who have the
knowledge or experience to teach them. This proposal aims to directly tackle the
tutoring industry while deepening the knowledge of all students.

## State of the Art

There are many platforms that give out online courses, tutoring services, and
educational resources. However, most of these platforms only focus on monetized
one-way learning, as opposed to a symbiotic relationship where both participants
are learners. Finding people who are interested in exchanging skills for another
requires people to manually search through social media groups or forums without
an organized matching system.

## Solution

Our solution is to create a Peer Skill Exchange Platform where users can share
skills they already know and find others who can help them learn something new.
The platform will connect users based on the skills they can offer and the skills
they are interested in learning. Rather than relying only on tutoring (paid or
otherwise), or on courses, users can exchange their skills and knowledge with one
another in a mutually beneficial exchange. Furthermore, students teaching each
other has a protégé effect (a proven psychological phenomenon) in which it helps
deepen the understanding of the subject for both participants.

## Stack

- **Rust + Axum** — server backend.
- **MiniJinja** — HTML templating, so front-end work does not require writing
  Rust.
- **HTMX** — server-side rendering. Reduces client-side JavaScript, which is a
  major contributor to First Paint time, by having the server produce the final
  HTML and letting HTMX inject it.
  ([why JS bloat matters](https://tonsky.me/blog/js-bloat),
  [why HTMX helps](https://www.reddit.com/r/htmx/comments/1bg621p/comment/kv7oo6i))
- **Tailwind** — CSS styling.
- **sqlx** — accounts stored in SQLite or Postgres, selected at runtime.
- **tower-sessions + Argon2** — cookie sessions and password hashing.

## Deployment

Clone the GitHub repository locally, use GitHub Actions for automated checks, and
test changes with relative ease.

---

## How the proposal maps onto what is built

This section tracks the proposal against the code. See the
[README](../README.md) for how to run and extend it.

| Proposal element | Where it lives | Status |
| --- | --- | --- |
| Users list skills they can offer | `User.teaching` in `src/models.rs` | Built |
| Users list skills they want | `User.learning` in `src/models.rs` | Built |
| Organized matching system | `Swap::between` in `src/models.rs` | Built |
| Mutually beneficial exchange | Matching requires overlap in **both** directions | Built |
| Search instead of trawling forums | `/browse` with live HTMX search | Built |
| Reduced client-side JavaScript | HTMX only; no bundler, no framework | Built |
| Easy local testing | `cargo run` plus one Tailwind script | Built |
| GitHub Actions | `.github/workflows/ci.yml` | Built |
| Accounts and profiles owned by a user | `src/auth.rs`, `src/routes/auth.rs` | Built |
| Contacting or scheduling with a match | — | Not started |
| Durable storage | `src/store.rs` over SQLite or Postgres | Built |
| Editing a profile after signup | — | Not started |

The design decision worth calling out: a match is only shown when the exchange
works both ways. A user who could teach you something but wants nothing you offer
is not a swap — that is ordinary tutoring, which is the thing the proposal is
reacting against.
