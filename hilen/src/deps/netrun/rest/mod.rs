mod client;
mod method;
mod request;
mod response;
mod rest_api;
mod simple;

pub(crate) use client::client;
pub use method::Method;
pub use request::Request;
pub use response::Response;
pub use rest_api::RestAPI;
pub use simple::*;

#[cfg(test)]
mod test {

    #[cfg(not_wasm)]
    mod not_wasm_test {

        use anyhow::Result;
        use pretty_assertions::assert_eq;

        use crate::deps::netrun::{
            rest::{Request, RestAPI, get},
            test_server::{User, start_test_server},
        };

        #[test]
        fn size_of_none() {
            const SIZE_OF_NONE: usize = size_of::<()>();
            assert_eq!(SIZE_OF_NONE, 0);
        }

        /// `Request` holds a `&'static RestAPI` and the server port is only
        /// known once it is bound, so the api outlives the test on purpose.
        fn api(base_url: String) -> &'static RestAPI {
            Box::leak(Box::new(RestAPI::new(Box::leak(base_url.into_boxed_str()))))
        }

        #[tokio::test]
        async fn test_rest() -> Result<()> {
            let api = api(start_test_server().await);
            let request: Request<(), Vec<User>> = api.get("users");

            let users = request.await?;

            assert_eq!(users.len(), 10);
            assert_eq!(users[0].id, 1);
            assert_eq!(users[0].username, "user1");
            assert_eq!(users[0].email, "user1@example.com");

            Ok(())
        }

        #[tokio::test]
        async fn test_simple() -> Result<()> {
            let base_url = start_test_server().await;

            let users: Vec<User> = get(format!("{base_url}/users")).await?;

            assert_eq!(users.len(), 10);

            Ok(())
        }
    }

    #[cfg(wasm)]
    mod wasm_test {
        use serde::Deserialize;
        use wasm_bindgen_test::wasm_bindgen_test;

        use crate::deps::netrun::rest::{Request, RestAPI};

        wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

        #[derive(Debug, Deserialize)]
        struct User {}

        /// A browser cannot host the local test server, so this one still
        /// dials the public API. No lane runs it, it is a by hand check.
        static API: RestAPI = RestAPI::new("https://jsonplaceholder.typicode.com/");

        static USERS: Request<(), Vec<User>> = API.get("users");

        #[wasm_bindgen_test]
        async fn test_rest() {
            let users = USERS.await.expect("Failed to fetch users");

            assert_eq!(users.len(), 10);
        }
    }
}
