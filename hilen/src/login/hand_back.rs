//! How the end of a login comes back to this app, the client half of
//! `hilen-server/src/auth/hand_back.rs`. The server gives a one time code to
//! the app instance that started the login, and the poll needs that code next
//! to the verifier. So a person who sends a login link of their own to
//! somebody else gets no session when that somebody logs in with it.
//!
//! A native app listens on a free port of `127.0.0.1` while the login runs,
//! the browser on the same device is sent there with the code. A page in a
//! browser reads the code from the `localStorage` of its origin, where the
//! last page of the login wrote it, so the page and the server need 1 origin.

use anyhow::Result;

/// The way back of 1 login. Dropping it ends the wait for its code.
pub(crate) struct HandBack {
    #[cfg(not_wasm)]
    listener: loopback::Listener,
    #[cfg(wasm)]
    page:     page::Page,
}

impl HandBack {
    /// Opens the way back for a login against `server` with this challenge.
    pub(crate) fn open(server: &str, challenge: &str) -> Result<Self> {
        #[cfg(not_wasm)]
        {
            Ok(Self {
                listener: loopback::Listener::open(server, challenge)?,
            })
        }

        #[cfg(wasm)]
        {
            Ok(Self {
                page: page::Page::open(server, challenge)?,
            })
        }
    }

    /// The part of the start link that names the way back.
    pub(crate) fn query(&self) -> String {
        #[cfg(not_wasm)]
        {
            format!("port={}", self.listener.port())
        }

        #[cfg(wasm)]
        {
            "page=1".to_owned()
        }
    }

    /// A code came back and waits to be taken.
    pub(crate) fn has_code(&self) -> bool {
        #[cfg(not_wasm)]
        {
            self.listener.has_code()
        }

        #[cfg(wasm)]
        {
            self.page.has_code()
        }
    }

    /// The code that came back last, once.
    pub(crate) fn take_code(&self) -> Option<String> {
        #[cfg(not_wasm)]
        {
            self.listener.take_code()
        }

        #[cfg(wasm)]
        {
            self.page.take_code()
        }
    }
}

/// A code of the server is 64 hex digits. Anything else that reaches the
/// port, like a request some web page sent to guess ports, is no code.
fn is_code(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(not_wasm)]
mod loopback {
    use std::{
        net::{Ipv4Addr, TcpListener as StdListener},
        sync::Arc,
        time::Duration,
    };

    use anyhow::{Context, Result, bail};
    use parking_lot::Mutex;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
        time::timeout,
    };

    use super::is_code;

    /// A request bigger than this is no browser coming back.
    const MAX_REQUEST: usize = 8192;
    /// A connection that sends no request in this time is dropped.
    const READ_SECONDS: u64 = 5;

    pub(super) struct Listener {
        port: u16,
        code: Arc<Mutex<Option<String>>>,
        task: JoinHandle<()>,
    }

    impl Listener {
        /// Binds a free port on `127.0.0.1` only, nothing from another
        /// machine reaches it. Needs the tokio runtime of the engine.
        pub(super) fn open(server: &str, challenge: &str) -> Result<Self> {
            let listener =
                StdListener::bind((Ipv4Addr::LOCALHOST, 0)).context("no free port for the login")?;
            listener.set_nonblocking(true)?;
            let port = listener.local_addr()?.port();
            let listener = TcpListener::from_std(listener)?;

            let code = Arc::new(Mutex::new(None));
            let task = tokio::spawn(serve(listener, code.clone(), format!("{server}/auth/done")));
            let short = challenge.get(..8).unwrap_or(challenge);
            log::debug!("the login {short} waits for its code on port {port}");

            Ok(Self { port, code, task })
        }

        pub(super) const fn port(&self) -> u16 {
            self.port
        }

        pub(super) fn has_code(&self) -> bool {
            self.code.lock().is_some()
        }

        pub(super) fn take_code(&self) -> Option<String> {
            self.code.lock().take()
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            self.task.abort();
            log::debug!("the login stopped listening on port {}", self.port);
        }
    }

    async fn serve(listener: TcpListener, code: Arc<Mutex<Option<String>>>, done_page: String) {
        loop {
            let stream = match listener.accept().await {
                Ok((stream, _)) => stream,
                Err(error) => {
                    log::warn!("the login port took no connection: {error}");
                    continue;
                }
            };
            match timeout(Duration::from_secs(READ_SECONDS), answer(stream, &done_page)).await {
                Ok(Ok(Some(found))) => {
                    log::info!("the code of the login came back");
                    *code.lock() = Some(found);
                }
                Ok(Ok(None)) => log::debug!("a request with no login code reached the login port"),
                Ok(Err(error)) => log::warn!("a request to the login port failed: {error}"),
                Err(_) => log::debug!("a connection to the login port sent nothing in time"),
            }
        }
    }

    /// Reads 1 request and answers it. A browser with the code is sent on to
    /// the last page of the server.
    async fn answer(mut stream: TcpStream, done_page: &str) -> Result<Option<String>> {
        let mut request = Vec::new();
        let mut chunk = [0; 1024];
        while !request.windows(4).any(|end| end == b"\r\n\r\n") {
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.len() > MAX_REQUEST {
                bail!("the request is over {MAX_REQUEST} bytes");
            }
        }

        let code = code_of(&String::from_utf8_lossy(&request));
        let reply = if code.is_some() {
            format!(
                "HTTP/1.1 303 See Other\r\nLocation: {done_page}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
        } else {
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
        };
        stream.write_all(reply.as_bytes()).await?;
        stream.shutdown().await?;
        Ok(code)
    }

    /// The code of `GET /?code=<code> HTTP/1.1`.
    pub(super) fn code_of(request: &str) -> Option<String> {
        let target = request.lines().next()?.strip_prefix("GET ")?.split(' ').next()?;
        let query = target.strip_prefix("/?")?;
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("code="))
            .filter(|code| is_code(code))
            .map(str::to_owned)
    }
}

#[cfg(wasm)]
mod page {
    use anyhow::{Result, anyhow, bail};
    use web_sys::{Storage, window};

    pub(super) struct Page {
        key: String,
    }

    impl Page {
        /// The last page of the login is a page of the server, it can write
        /// only into the storage of its own origin.
        pub(super) fn open(server: &str, challenge: &str) -> Result<Self> {
            let origin = window()
                .ok_or_else(|| anyhow!("no window"))?
                .location()
                .origin()
                .map_err(|error| anyhow!("the page has no origin: {error:?}"))?;
            if origin.trim_end_matches('/') != server {
                bail!(
                    "a login in a browser needs the page on the origin of its server {server}, this page is on {origin}"
                );
            }
            // The same key as `HandBack::storage_key` of the server.
            Ok(Self {
                key: format!("hilen-login-{challenge}"),
            })
        }

        pub(super) fn has_code(&self) -> bool {
            self.read().is_some()
        }

        pub(super) fn take_code(&self) -> Option<String> {
            let code = self.read()?;
            if let Err(error) = storage().and_then(|storage| {
                storage
                    .remove_item(&self.key)
                    .map_err(|error| anyhow!("failed to clear localStorage: {error:?}"))
            }) {
                log::warn!("the code of the login stays in localStorage: {error}");
            }
            log::info!("the code of the login came back");
            Some(code)
        }

        fn read(&self) -> Option<String> {
            match storage().and_then(|storage| {
                storage
                    .get_item(&self.key)
                    .map_err(|error| anyhow!("failed to read localStorage: {error:?}"))
            }) {
                Ok(code) => code.filter(|code| super::is_code(code)),
                Err(error) => {
                    log::warn!("the code of the login cannot be read: {error}");
                    None
                }
            }
        }
    }

    fn storage() -> Result<Storage> {
        window()
            .ok_or_else(|| anyhow!("no window"))?
            .local_storage()
            .map_err(|error| anyhow!("localStorage is blocked: {error:?}"))?
            .ok_or_else(|| anyhow!("no localStorage"))
    }
}

#[cfg(all(test, not_wasm))]
mod test {
    use anyhow::Result;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    use super::{HandBack, loopback::code_of};

    fn code() -> String {
        "ab".repeat(32)
    }

    #[test]
    fn only_a_get_of_the_root_with_a_code_counts() {
        let code = code();
        assert_eq!(
            code_of(&format!("GET /?code={code} HTTP/1.1\r\nHost: x\r\n\r\n")),
            Some(code.clone())
        );
        assert_eq!(
            code_of(&format!("GET /?a=1&code={code} HTTP/1.1\r\n\r\n")),
            Some(code.clone())
        );
        assert_eq!(code_of(&format!("POST /?code={code} HTTP/1.1\r\n\r\n")), None);
        assert_eq!(
            code_of(&format!("GET /favicon.ico?code={code} HTTP/1.1\r\n\r\n")),
            None
        );
        assert_eq!(code_of("GET /?code=short HTTP/1.1\r\n\r\n"), None);
        assert_eq!(code_of("GET / HTTP/1.1\r\n\r\n"), None);
    }

    async fn send(port: u16, request: &str) -> Result<String> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(request.as_bytes()).await?;
        let mut reply = String::new();
        stream.read_to_string(&mut reply).await?;
        Ok(reply)
    }

    /// The browser comes back to the port with the code and is sent on to
    /// the last page of the server. A request with no code changes nothing.
    #[tokio::test]
    async fn the_code_that_comes_back_to_the_port_is_kept_once() -> Result<()> {
        let back = HandBack::open("https://app.example.com", "challenge")?;
        let port = back.query().trim_start_matches("port=").parse()?;

        let reply = send(port, "GET /favicon.ico HTTP/1.1\r\n\r\n").await?;
        assert!(reply.starts_with("HTTP/1.1 404"));
        assert!(!back.has_code());

        let reply = send(
            port,
            &format!("GET /?code={} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", code()),
        )
        .await?;
        assert!(reply.starts_with("HTTP/1.1 303"));
        assert!(reply.contains("Location: https://app.example.com/auth/done\r\n"));
        assert!(back.has_code());
        assert_eq!(back.take_code(), Some(code()));
        assert_eq!(back.take_code(), None);

        drop(back);
        Ok(())
    }
}
