//! The security headers on every answer of a backend.
//!
//! [`serve`](crate::serve) puts [`base`] on every router, so each answer
//! carries `nosniff`, a `Referrer-Policy`, a frame block, and a closed
//! `Content-Security-Policy` when the route set none. A backend that serves a
//! trunk page adds [`PageCsp`] on top, which gives every html answer a policy
//! the page can run under, its own scripts and wasm and the inline scripts of
//! its `index.html` by their hashes, and `Cross-Origin-Opener-Policy`.

use axum::{
    Router,
    http::{HeaderMap, HeaderName, HeaderValue, header},
    middleware::map_response,
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rust_embed::RustEmbed;
use sha2::{Digest, Sha256};

/// The policy of an answer that is not a page, like json or a file. It loads
/// nothing and runs nothing, and no other site can frame it.
const CLOSED_CSP: &str = "default-src 'none'; frame-ancestors 'none'";

/// A page another site opened with `window.open` gets no handle back to that
/// site, and that site none to the page, so it cannot send the tab of the page
/// to a fake sign in page later.
const OPENER_POLICY: HeaderName = HeaderName::from_static("cross-origin-opener-policy");

/// Puts the headers every answer needs, unless the route set them itself.
pub(crate) fn base(router: Router) -> Router {
    router.layer(map_response(|mut response: Response| async move {
        let headers = response.headers_mut();
        set_missing(
            headers,
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        set_missing(
            headers,
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
        set_missing(headers, header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
        set_missing(
            headers,
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CLOSED_CSP),
        );
        response
    }))
}

fn set_missing(headers: &mut HeaderMap, name: header::HeaderName, value: HeaderValue) {
    if !headers.contains_key(&name) {
        headers.insert(name, value);
    }
}

/// The `Content-Security-Policy` of a trunk page.
///
/// ```ignore
/// let app = api_routes().merge(web_mount::<Web, _>());
/// let app = match PageCsp::for_dist::<Web>() {
///     Some(csp) => csp.layer(app),
///     None => app,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PageCsp {
    script_hashes: Vec<String>,
    connect:       Vec<String>,
    images:        Vec<String>,
}

impl PageCsp {
    /// The policy for the page of the embedded dist. Every inline script of
    /// its `index.html`, like the boot script and the loader of trunk, is
    /// allowed by its hash, so a script that gets into the page any other
    /// way does not run. `None` when the dist has no `index.html`, like a
    /// build of a test that has no page.
    pub fn for_dist<A: RustEmbed>() -> Option<Self> {
        let Some(index) = A::get("index.html") else {
            tracing::warn!("the embedded dist has no index.html, its pages get no page policy");
            return None;
        };
        let html = String::from_utf8_lossy(&index.data);
        Some(Self::for_html(&html))
    }

    pub(crate) fn for_html(html: &str) -> Self {
        let script_hashes = inline_scripts(html)
            .map(|script| format!("'sha256-{}'", STANDARD.encode(Sha256::digest(script.as_bytes()))))
            .collect::<Vec<_>>();
        tracing::info!(
            "the page policy allows {} inline scripts by hash",
            script_hashes.len()
        );
        Self {
            script_hashes,
            connect: Vec::new(),
            images: Vec::new(),
        }
    }

    /// Lets the page `fetch` from `origin`, like `https://api.example.com`.
    /// A page that downloads the account picture of a login provider names
    /// its host here, like `https://*.googleusercontent.com`.
    #[must_use]
    pub fn connect(mut self, origin: &str) -> Self {
        self.connect.push(origin.to_string());
        self
    }

    /// Lets the page show pictures from `origin` in an `img` element.
    #[must_use]
    pub fn images(mut self, origin: &str) -> Self {
        self.images.push(origin.to_string());
        self
    }

    /// The header value.
    ///
    /// # Panics
    ///
    /// When an origin given to [`Self::connect`] or [`Self::images`] has a
    /// character a header cannot hold.
    pub fn value(&self) -> HeaderValue {
        let join = |list: &[String]| {
            let mut joined = String::new();
            for item in list {
                joined.push(' ');
                joined.push_str(item);
            }
            joined
        };
        let policy = format!(
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'{scripts}; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:{images}; connect-src 'self'{connect}; font-src 'self' data:; \
             worker-src 'self' blob:; object-src 'none'; base-uri 'self'; form-action 'self'; \
             frame-ancestors 'none'",
            scripts = join(&self.script_hashes),
            images = join(&self.images),
            connect = join(&self.connect),
        );
        HeaderValue::from_str(&policy)
            .expect("an origin of the page policy has a character a header cannot hold")
    }

    /// Gives every html answer of `router` this policy and
    /// `Cross-Origin-Opener-Policy: same-origin`, unless the route set its own.
    pub fn layer<S>(self, router: Router<S>) -> Router<S>
    where S: Clone + Send + Sync + 'static {
        let value = self.value();
        router.layer(map_response(move |mut response: Response| {
            let value = value.clone();
            async move {
                let is_page = response
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|kind| kind.to_str().ok())
                    .is_some_and(|kind| kind.starts_with("text/html"));
                if is_page {
                    let headers = response.headers_mut();
                    set_missing(headers, header::CONTENT_SECURITY_POLICY, value);
                    set_missing(headers, OPENER_POLICY, HeaderValue::from_static("same-origin"));
                }
                response
            }
        }))
    }
}

/// The text of every `script` element of `html` that has no `src`.
fn inline_scripts(html: &str) -> impl Iterator<Item = &str> {
    let mut rest = html;
    std::iter::from_fn(move || {
        loop {
            let start = rest.find("<script")?;
            let after = &rest[start..];
            let open_end = after.find('>')?;
            let open = &after[..open_end];
            let body_start = start + open_end + 1;
            let body_len = rest[body_start..].find("</script>")?;
            let body = &rest[body_start..body_start + body_len];
            rest = &rest[body_start + body_len..];
            if !open.contains(" src=") {
                return Some(body);
            }
        }
    })
}

#[cfg(test)]
mod test {
    use axum::{
        Router,
        body::Body,
        http::{Request, header},
        response::Html,
        routing::get,
    };
    use base64::{Engine, engine::general_purpose::STANDARD};
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;

    use super::{PageCsp, base, inline_scripts};

    const PAGE: &str = r#"<html><head><script>boot()</script><script type="module" src="/app.js"></script>
<script type="module">import init from '/app.js'; init();</script></head><body></body></html>"#;

    #[test]
    fn only_inline_scripts_are_found() {
        let found = inline_scripts(PAGE).collect::<Vec<_>>();
        assert_eq!(found, ["boot()", "import init from '/app.js'; init();"]);
    }

    #[test]
    fn the_page_policy_holds_the_hash_of_each_inline_script() {
        let policy = PageCsp::for_html(PAGE).connect("https://api.example.com").value();
        let policy = policy.to_str().expect("ascii");
        let boot = format!("'sha256-{}'", STANDARD.encode(Sha256::digest(b"boot()")));
        assert!(policy.contains(&boot), "{policy}");
        assert!(policy.contains("'wasm-unsafe-eval'"), "{policy}");
        assert!(
            policy.contains("connect-src 'self' https://api.example.com;"),
            "{policy}"
        );
        assert!(policy.contains("frame-ancestors 'none'"), "{policy}");
        assert!(!policy.contains("script-src 'self' 'unsafe-inline'"), "{policy}");
    }

    async fn headers_of(router: Router, path: &str) -> axum::http::HeaderMap {
        let response = router
            .oneshot(Request::builder().uri(path).body(Body::empty()).expect("request"))
            .await
            .expect("answer");
        response.headers().clone()
    }

    fn app() -> Router {
        let routes = Router::new()
            .route("/page", get(|| async { Html("<p>page</p>") }))
            .route("/api", get(|| async { "{}" }))
            .route(
                "/file",
                get(|| async { ([(header::CONTENT_SECURITY_POLICY, "sandbox")], "bytes") }),
            );
        base(PageCsp::for_html(PAGE).layer(routes))
    }

    #[tokio::test]
    async fn a_page_gets_the_page_policy_and_the_rest_the_closed_one() {
        let page = headers_of(app(), "/page").await;
        let csp = page[header::CONTENT_SECURITY_POLICY].to_str().expect("ascii");
        assert!(csp.starts_with("default-src 'self'"), "{csp}");
        assert_eq!(page[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(page[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(page[header::REFERRER_POLICY], "strict-origin-when-cross-origin");
        assert_eq!(page["cross-origin-opener-policy"], "same-origin");

        let api = headers_of(app(), "/api").await;
        assert_eq!(
            api[header::CONTENT_SECURITY_POLICY],
            "default-src 'none'; frame-ancestors 'none'"
        );
        assert_eq!(api[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert!(!api.contains_key("cross-origin-opener-policy"));
    }

    /// A page that never fetches from another site lets no script send data
    /// there.
    #[test]
    fn the_page_policy_fetches_only_from_its_own_origin() {
        let policy = PageCsp::for_html(PAGE).value();
        let policy = policy.to_str().expect("ascii");
        assert!(policy.contains("connect-src 'self';"), "{policy}");
        assert!(!policy.contains("googleusercontent"), "{policy}");
        assert!(!policy.contains("discordapp"), "{policy}");
    }

    #[tokio::test]
    async fn a_policy_the_route_set_stays() {
        let file = headers_of(app(), "/file").await;
        assert_eq!(file[header::CONTENT_SECURITY_POLICY], "sandbox");
    }
}
