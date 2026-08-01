mod client;
mod method;
mod request;
mod response;
mod rest_api;
mod simple;

pub use method::Method;
pub use request::Request;
pub use response::Response;
pub use rest_api::RestAPI;
pub use simple::*;

#[cfg(test)]
mod test {

    use serde::Deserialize;

    use crate::rest::{Request, RestAPI};

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct User {
        id:       u32,
        username: String,
        email:    String,
    }

    static API: RestAPI = RestAPI::new("https://jsonplaceholder.typicode.com/");

    static USERS: Request<(), Vec<User>> = API.get("users");

    #[cfg(not_wasm)]
    mod not_wasm_test {

        use anyhow::Result;
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::rest::get;

        #[test]
        fn size_of_none() {
            const SIZE_OF_NONE: usize = size_of::<()>();
            assert_eq!(SIZE_OF_NONE, 0);
        }

        #[tokio::test]
        async fn test_rest() -> Result<()> {
            let users = USERS.await?;

            assert_eq!(users.len(), 10);

            Ok(())
        }

        #[tokio::test]
        async fn test_simple() -> Result<()> {
            let users: Vec<User> = get("https://jsonplaceholder.typicode.com/users").await?;

            assert_eq!(users.len(), 10);

            Ok(())
        }
    }

    #[cfg(wasm)]
    mod wasm_test {
        use wasm_bindgen_test::wasm_bindgen_test;

        use super::*;

        wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

        #[wasm_bindgen_test]
        async fn test_rest() {
            let users = USERS.await.expect("Failed to fetch users");

            assert_eq!(users.len(), 10);
        }
    }
}
