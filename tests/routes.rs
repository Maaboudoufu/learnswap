//! End-to-end tests that drive the real router in-process -- no network, no
//! browser. Each test gets its own in-memory SQLite database, migrated and
//! seeded from scratch, so they are independent and run in milliseconds.
//!
//! To run these against Postgres instead:
//!     TEST_DATABASE_URL=postgres://user:pass@localhost/learnswap_test cargo test
//! Note that Postgres runs share one database, so run them single-threaded:
//!     cargo test -- --test-threads=1

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use learnswap::models::{Skill, User};
use learnswap::sqlx;
use learnswap::uuid::Uuid;
use learnswap::{AppState, Store, router};
use tower::ServiceExt;

/// A fresh, seeded store. In-memory SQLite unless TEST_DATABASE_URL overrides.
async fn seeded_store() -> Store {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
    let store = Store::connect(&url)
        .await
        .expect("connect to test database");
    if url.starts_with("postgres") {
        // Shared database: start from a known-empty table.
        sqlx::query("DELETE FROM users")
            .execute(store.pool())
            .await
            .expect("clear users");
    }
    store.seed_if_empty().await.expect("seed");
    store
}

async fn seeded_app() -> Router {
    router(AppState::new(seeded_store().await))
}

async fn read_body(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Sends a GET and returns the status plus the body as a string.
async fn get(app: Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    (status, read_body(response).await)
}

/// Sends a form POST. `htmx` decides whether the HX-Request header is set.
async fn post_form(
    app: Router,
    uri: &str,
    body: &'static str,
    htmx: bool,
) -> axum::response::Response {
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if htmx {
        req = req.header("hx-request", "true");
    }
    app.oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

// ------------------------------------------------------------------- browsing

#[tokio::test]
async fn health_check_responds_ok() {
    let (status, body) = get(seeded_app().await, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn landing_page_renders() {
    let (status, body) = get(seeded_app().await, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Teach one thing"));
    assert!(body.contains("Ana Ruiz"));
}

#[tokio::test]
async fn browse_lists_every_member() {
    let (status, body) = get(seeded_app().await, "/browse").await;
    assert_eq!(status, StatusCode::OK);
    for name in ["Ana Ruiz", "Marcus Bell", "Priya Nair"] {
        assert!(body.contains(name), "expected {name} on the browse page");
    }
}

#[tokio::test]
async fn search_filters_by_skill_and_is_case_insensitive() {
    let (status, body) = get(seeded_app().await, "/browse/results?q=GUITAR").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Marcus Bell"));
    assert!(!body.contains("Tom Okafor"));
}

#[tokio::test]
async fn search_with_no_hits_shows_empty_state() {
    let (status, body) = get(seeded_app().await, "/browse/results?q=underwater+basketry").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("No one matches that yet"));
}

#[tokio::test]
async fn search_wildcards_are_treated_as_literal_text() {
    // A bare "%" would match every row if it reached LIKE unescaped.
    let (status, body) = get(seeded_app().await, "/browse/results?q=%25").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("No one matches that yet"), "got: {body}");
}

#[tokio::test]
async fn profile_lists_only_two_way_matches() {
    let store = seeded_store().await;
    let ana = find_by_name(&store, "Ana Ruiz").await;
    let app = router(AppState::new(store));

    let (status, body) = get(app, &format!("/members/{}", ana.id)).await;
    assert_eq!(status, StatusCode::OK);
    // Ana teaches Rust and wants Guitar; Marcus is the mirror image of that.
    assert!(body.contains("Marcus Bell"));
    // Lena wants pottery from Ana but teaches nothing Ana asked for.
    assert!(!body.contains("Lena Fischer"));
}

#[tokio::test]
async fn a_profile_page_never_leaks_an_email_address() {
    let store = seeded_store().await;
    let ana = find_by_name(&store, "Ana Ruiz").await;
    let app = router(AppState::new(store));

    let (_, body) = get(app, &format!("/members/{}", ana.id)).await;
    assert!(
        !body.contains("ana@example.edu"),
        "email leaked into profile"
    );
    assert!(!body.contains("@example.edu"));
}

#[tokio::test]
async fn a_page_never_leaks_a_password_hash() {
    let (_, body) = get(seeded_app().await, "/browse").await;
    assert!(!body.contains("$argon2"), "password hash leaked into page");
}

#[tokio::test]
async fn unknown_member_returns_404_page() {
    let missing = Uuid::nil();
    let (status, body) = get(seeded_app().await, &format!("/members/{missing}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("could not find that page"));
}

#[tokio::test]
async fn unknown_path_returns_404_page() {
    let (status, body) = get(seeded_app().await, "/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("could not find that page"));
}

// --------------------------------------------------------------- registration

const NEW_ACCOUNT: &str = "email=dana@example.edu&password=swapswap1&name=Dana+Lin\
                           &headline=&bio=&teaching=Spanish&learning=Rust";

#[tokio::test]
async fn registering_over_htmx_signs_you_in_and_redirects() {
    let response = post_form(seeded_app().await, "/register", NEW_ACCOUNT, true).await;

    assert_eq!(response.status(), StatusCode::OK);
    // HTMX cannot follow a 303 into a navigation, so we send HX-Redirect.
    let target = response.headers()["HX-Redirect"].to_str().unwrap();
    assert!(target.starts_with("/members/"));
    // A session cookie means the new account is already logged in.
    assert!(response.headers().contains_key(header::SET_COOKIE));
}

#[tokio::test]
async fn plain_form_registration_redirects_to_the_new_profile() {
    let response = post_form(seeded_app().await, "/register", NEW_ACCOUNT, false).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers()[header::LOCATION].to_str().unwrap();
    assert!(location.starts_with("/members/"));
}

#[tokio::test]
async fn registration_requires_a_skill_in_each_direction() {
    let response = post_form(
        seeded_app().await,
        "/register",
        "email=dana@example.edu&password=swapswap1&name=Dana+Lin&headline=&bio=&teaching=&learning=",
        true,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_body(response).await;
    assert!(body.contains("List at least one skill you can teach."));
    assert!(body.contains("List at least one skill you want to learn."));
    // What the user typed survives the round trip.
    assert!(body.contains("Dana Lin"));
}

#[tokio::test]
async fn registration_rejects_a_short_password_and_never_echoes_it() {
    let response = post_form(
        seeded_app().await,
        "/register",
        "email=dana@example.edu&password=short&name=Dana+Lin&headline=&bio=&teaching=Spanish&learning=Rust",
        true,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_body(response).await;
    assert!(body.contains("at least 8 characters"));
    assert!(
        !body.contains("short"),
        "password was echoed back into the form"
    );
}

#[tokio::test]
async fn registration_rejects_a_duplicate_email() {
    let response = post_form(
        seeded_app().await,
        "/register",
        // Ana is in the seed data.
        "email=ANA@example.edu&password=swapswap1&name=Impostor&headline=&bio=&teaching=Rust&learning=Guitar",
        true,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_body(response).await;
    assert!(body.contains("already registered"), "got: {body}");
}

// ---------------------------------------------------------------------- login

#[tokio::test]
async fn correct_credentials_log_you_in() {
    let response = post_form(
        seeded_app().await,
        "/login",
        "email=ana@example.edu&password=learnswap",
        true,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("HX-Redirect"));
    assert!(response.headers().contains_key(header::SET_COOKIE));
}

#[tokio::test]
async fn login_email_is_case_insensitive() {
    let response = post_form(
        seeded_app().await,
        "/login",
        "email=ANA@EXAMPLE.EDU&password=learnswap",
        true,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("HX-Redirect"));
}

#[tokio::test]
async fn a_wrong_password_is_rejected() {
    let response = post_form(
        seeded_app().await,
        "/login",
        "email=ana@example.edu&password=not-the-password",
        true,
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(!response.headers().contains_key("HX-Redirect"));
}

#[tokio::test]
async fn an_unknown_email_gives_the_same_message_as_a_wrong_password() {
    let unknown = post_form(
        seeded_app().await,
        "/login",
        "email=nobody@example.edu&password=learnswap",
        true,
    )
    .await;
    let wrong = post_form(
        seeded_app().await,
        "/login",
        "email=ana@example.edu&password=wrong-one",
        true,
    )
    .await;

    assert_eq!(unknown.status(), wrong.status());

    // The bodies differ only by the email each one echoes back into the form.
    // What must match is the error message: it may not say which of the two
    // fields was wrong, or the page becomes an address-enumeration oracle.
    const MESSAGE: &str = "That email and password do not match an account.";
    let unknown_body = read_body(unknown).await;
    let wrong_body = read_body(wrong).await;
    assert!(unknown_body.contains(MESSAGE), "got: {unknown_body}");
    assert!(wrong_body.contains(MESSAGE), "got: {wrong_body}");
    for body in [&unknown_body, &wrong_body] {
        assert!(!body.to_lowercase().contains("no such"));
        assert!(!body.to_lowercase().contains("not registered"));
        assert!(!body.to_lowercase().contains("incorrect password"));
    }
}

#[tokio::test]
async fn a_protected_route_redirects_anonymous_visitors_to_login() {
    let (status, _) = get(seeded_app().await, "/me").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
}

// ----------------------------------------------------------------- unit-ish

#[tokio::test]
async fn passwords_are_stored_hashed_not_in_the_clear() {
    let store = seeded_store().await;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users LIMIT 1")
        .fetch_one(store.pool())
        .await
        .expect("read a hash");
    assert!(hash.starts_with("$argon2"), "got: {hash}");
    assert!(!hash.contains("learnswap"));
}

#[tokio::test]
async fn authenticate_accepts_the_seed_password_and_rejects_others() {
    let store = seeded_store().await;
    assert!(
        store
            .authenticate("ana@example.edu", "learnswap")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .authenticate("ana@example.edu", "wrong")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn seeding_is_idempotent() {
    let store = seeded_store().await;
    let before = store.count().await.unwrap();
    assert!(!store.seed_if_empty().await.unwrap(), "seeded twice");
    assert_eq!(store.count().await.unwrap(), before);
}

#[test]
fn duplicate_and_blank_skill_tags_are_dropped() {
    let skills = Skill::parse_list("Rust, rust ,  , Pottery");
    let labels: Vec<&str> = skills.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["Rust", "Pottery"]);
}

async fn find_by_name(store: &Store, name: &str) -> User {
    store
        .all()
        .await
        .expect("load users")
        .into_iter()
        .find(|u| u.name == name)
        .unwrap_or_else(|| panic!("seed data contains {name}"))
}
