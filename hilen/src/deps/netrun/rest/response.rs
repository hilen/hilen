use std::fmt::Debug;

use reqwest::StatusCode;

#[derive(Debug)]
pub struct Response {
    #[allow(dead_code)]
    pub url:    String,
    #[allow(dead_code)]
    pub status: StatusCode,
    pub body:   String,
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.status == 200
    }
}
