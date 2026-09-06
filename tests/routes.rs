//! End-to-end tests that drive the real router in-process -- no network, no
//! browser. These are the tests to copy when you add a route.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use learnswap::{AppState, Store, models::Member, router};
use tower::ServiceExt;

fn app_with_seed_data() -> Router {
    router(AppState::new(Store::with_seed_data()))
}

/// Sends a request and returns the status plus the body as a string.
async fn get(app: Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn health_check_responds_ok() {
    let (status, body) = get(app_with_seed_data(), "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn landing_page_renders() {
    let (status, body) = get(app_with_seed_data(), "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Teach one thing"));
    // Seeded members should show up in the featured-swaps section.
    assert!(body.contains("Ana Ruiz"));
}

#[tokio::test]
async fn browse_lists_every_member() {
    let (status, body) = get(app_with_seed_data(), "/browse").await;
    assert_eq!(status, StatusCode::OK);
    for name in ["Ana Ruiz", "Marcus Bell", "Priya Nair"] {
        assert!(body.contains(name), "expected {name} on the browse page");
    }
}

#[tokio::test]
async fn search_filters_by_skill_and_is_case_insensitive() {
    let (status, body) = get(app_with_seed_data(), "/browse/results?q=GUITAR").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Marcus Bell"));
    assert!(!body.contains("Tom Okafor"));
}

#[tokio::test]
async fn search_with_no_hits_shows_empty_state() {
    let (status, body) = get(
        app_with_seed_data(),
        "/browse/results?q=underwater+basketry",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("No one matches that yet"));
}

#[tokio::test]
async fn profile_lists_only_two_way_matches() {
    let store = Store::with_seed_data();
    let ana = store
        .all()
        .into_iter()
        .find(|m| m.name == "Ana Ruiz")
        .expect("seed data contains Ana");
    let app = router(AppState::new(store));

    let (status, body) = get(app, &format!("/members/{}", ana.id)).await;
    assert_eq!(status, StatusCode::OK);
    // Ana teaches Rust and wants Guitar; Marcus is the mirror image of that.
    assert!(body.contains("Marcus Bell"));
    // Lena wants pottery from Ana but teaches nothing Ana asked for.
    assert!(!body.contains("Lena Fischer"));
}

#[tokio::test]
async fn unknown_member_returns_404_page() {
    let missing = uuid_like_but_absent();
    let (status, body) = get(app_with_seed_data(), &format!("/members/{missing}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("could not find that page"));
}

#[tokio::test]
async fn unknown_path_returns_404_page() {
    let (status, body) = get(app_with_seed_data(), "/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("could not find that page"));
}

#[tokio::test]
async fn joining_over_htmx_returns_the_success_fragment() {
    let app = app_with_seed_data();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/join")
                .header("hx-request", "true")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(
                    "name=Dana+Lin&headline=&bio=&teaching=Spanish&learning=Rust",
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();

    assert!(body.contains("You are on the board, Dana Lin"));
    // Dana teaches Spanish and wants Rust -- Ana is exactly that trade.
    assert!(body.contains("Ana Ruiz"));
    // A fragment, not a whole document.
    assert!(!body.contains("<!doctype html>"));
}

#[tokio::test]
async fn join_without_a_skill_in_each_direction_is_rejected() {
    let app = app_with_seed_data();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/join")
                .header("hx-request", "true")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(
                    "name=Dana+Lin&headline=&bio=&teaching=&learning=",
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("List at least one skill you can teach."));
    assert!(body.contains("List at least one skill you want to learn."));
    // The name the user already typed survives the round trip.
    assert!(body.contains("Dana Lin"));
}

#[tokio::test]
async fn plain_form_post_redirects_to_the_new_profile() {
    let app = app_with_seed_data();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/join")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(
                    "name=Dana+Lin&headline=&bio=&teaching=Spanish&learning=Rust",
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers()[header::LOCATION].to_str().unwrap();
    assert!(location.starts_with("/members/"));
}

#[tokio::test]
async fn duplicate_and_blank_skill_tags_are_dropped() {
    let member = Member::new("T", "", "", "Rust, rust ,  , Pottery", "Guitar");
    let labels: Vec<&str> = member.teaching.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["Rust", "Pottery"]);
}

fn uuid_like_but_absent() -> &'static str {
    "00000000-0000-4000-8000-000000000000"
}
