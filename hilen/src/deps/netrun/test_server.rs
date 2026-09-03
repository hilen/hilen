#![cfg(all(test, not_wasm))]

//! A local stand in for the public JSON API and the file host the netrun
//! tests used to dial. It answers the same shapes on a random loopback
//! port, so the tests run offline and nothing can change under them.

use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct User {
    pub id:       u32,
    pub username: String,
    pub email:    String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NewPost {
    pub title:   String,
    #[serde(rename = "userId")]
    pub user_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PostPatch {
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Post {
    pub id:      u32,
    pub title:   String,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<u32>,
}

/// What a delete answers with, an object with no fields.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Empty {}

pub(crate) const FILE_SIZE: usize = 97_126;

/// Binds a free loopback port and returns the base url, no trailing slash.
pub(crate) async fn start_test_server() -> String {
    let app = Router::new()
        .route("/users", get(users))
        .route("/posts", post(create_post))
        .route("/posts/{id}", patch(patch_post).delete(delete_post))
        .route("/file", get(file));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind the test server");
    let address = listener.local_addr().expect("Failed to read the test server address");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("The test server died");
    });

    format!("http://{address}")
}

async fn users() -> Json<Vec<User>> {
    Json(
        (1..=10)
            .map(|id| User {
                id,
                username: format!("user{id}"),
                email: format!("user{id}@example.com"),
            })
            .collect(),
    )
}

/// 201 like the public API did, which proves every 2xx is accepted, not
/// only 200.
async fn create_post(Json(new): Json<NewPost>) -> (StatusCode, Json<Post>) {
    let post = Post {
        id:      101,
        title:   new.title,
        user_id: Some(new.user_id),
    };

    (StatusCode::CREATED, Json(post))
}

async fn patch_post(Path(id): Path<u32>, Json(patch): Json<PostPatch>) -> Json<Post> {
    Json(Post {
        id,
        title: patch.title,
        user_id: None,
    })
}

async fn delete_post() -> Json<Empty> {
    Json(Empty {})
}

async fn file() -> Vec<u8> {
    vec![0x5A; FILE_SIZE]
}
