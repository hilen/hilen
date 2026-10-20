//! The bridge to an app that runs as a page, like the dev package of an app
//! installed on an LG TV. A page cannot listen, it dials out with a
//! WebSocket, see "An installed TV app" in `docs/inspect.md`. The bridge
//! takes that socket, opens a normal inspect port for the page and
//! announces it like a native app, so every command of this tool and the
//! inspector reach the page with no change. The log lines of the page are
//! printed here and kept in a file.

use std::{env::temp_dir, fs::OpenOptions, io::Write, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use hilen::inspect::protocol::{AppCommand, announce};
use serde_json::{Value, from_str, to_vec};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    process::Command,
    select, spawn,
    sync::{mpsc, oneshot},
    time::{sleep, timeout},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// A request is a small command, as in the transport of an app.
const MAX_REQUEST_LEN: u32 = 64 * 1024 * 1024;
/// How long a page has to say which app it is.
const HELLO: Duration = Duration::from_secs(10);
/// The pause before the tunnel is opened again.
const TUNNEL_AGAIN: Duration = Duration::from_secs(3);

/// A request of a tool and where its answer goes.
type Request = (Vec<u8>, oneshot::Sender<Vec<u8>>);

/// Takes pages on `port` of this machine until it is stopped. With a
/// `tunnel`, an ssh destination like `root@192.168.0.155`, it also keeps a
/// reverse tunnel open, so the page dials `127.0.0.1:port` on its device
/// and lands here. `bind` is the own address ssh sends from.
pub(super) async fn run(port: u16, tunnel: Option<String>, bind: Option<String>) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("the port {port} is taken"))?;
    println!("waiting for pages on 127.0.0.1:{port}");
    if let Some(target) = tunnel {
        spawn(keep_tunnel(target, bind, port));
    }
    loop {
        let (stream, from) = listener.accept().await?;
        spawn(async move {
            if let Err(err) = serve_page(stream).await {
                println!("the page from {from} is gone: {err:#}");
            }
        });
    }
}

/// Keeps `ssh -R` open, a TV that sleeps or an ssh that drops ends it.
async fn keep_tunnel(target: String, bind: Option<String>, port: u16) {
    loop {
        let mut ssh = Command::new("ssh");
        ssh.args([
            "-N",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "ServerAliveInterval=10",
        ])
        .args(["-R", &format!("{port}:127.0.0.1:{port}")]);
        if let Some(bind) = &bind {
            ssh.args(["-o", &format!("BindAddress={bind}")]);
        }
        ssh.arg(&target).stdin(Stdio::null()).kill_on_drop(true);
        println!("tunnel to {target} opens, the page dials 127.0.0.1:{port} there");
        match ssh.status().await {
            Ok(status) => println!("the tunnel to {target} ended with {status}, it opens again"),
            Err(err) => println!("ssh did not start: {err}, it is tried again"),
        }
        sleep(TUNNEL_AGAIN).await;
    }
}

/// A frame the page sends on its own, not as an answer.
enum Pushed {
    Log(String, String),
    Hello(String, String),
    Other(String),
}

/// What a frame of the page is. A test report is an answer while a tool
/// waits for 1, the autorun of the tests pushes it too.
fn pushed(text: &str, asked: bool) -> Option<Pushed> {
    let value: Value = from_str(text).ok()?;
    let object = value.as_object()?;
    let field = |outer: &str, inner: &str| {
        object
            .get(outer)
            .and_then(|value| value.get(inner))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    if object.contains_key("Log") {
        return Some(Pushed::Log(field("Log", "level"), field("Log", "message")));
    }
    if object.contains_key("Hello") {
        return Some(Pushed::Hello(field("Hello", "app_id"), field("Hello", "name")));
    }
    if object.contains_key("FailureScreenshot") || (object.contains_key("TestResults") && !asked) {
        let kind = object.keys().next().cloned().unwrap_or_default();
        return Some(Pushed::Other(kind));
    }
    None
}

/// 1 page: waits for its hello, opens and announces its inspect port, and
/// carries requests to the page and answers back until the page goes.
async fn serve_page(stream: TcpStream) -> Result<()> {
    let socket = accept_async(stream).await.context("no WebSocket")?;
    let (mut to_page, mut from_page) = socket.split();

    let (app_id, name) = timeout(HELLO, async {
        while let Some(message) = from_page.next().await {
            if let Message::Text(text) = message? {
                match pushed(&text, false) {
                    Some(Pushed::Hello(app_id, name)) => return Ok((app_id, name)),
                    Some(Pushed::Log(level, line)) => println!("page {level} {line}"),
                    _ => {}
                }
            }
        }
        bail!("the page closed before its hello")
    })
    .await
    .context("the page said no hello, its engine is from before the bridge")??;

    let tools = TcpListener::bind(("0.0.0.0", 0)).await?;
    let port = tools.local_addr()?.port();
    let _announcement = announce(&app_id, &name, port)?;
    let log = log_path(&app_id);
    println!("{name} {app_id} is on port {port}, its log is {}", log.display());

    let (requests, mut asked) = mpsc::channel::<Request>(16);
    let accept = spawn(take_tools(tools, requests));

    let mut waiting: Option<oneshot::Sender<Vec<u8>>> = None;
    let result = loop {
        select! {
            request = asked.recv(), if waiting.is_none() => {
                let Some((data, answer)) = request else {
                    break Ok(());
                };
                let text = String::from_utf8(data)?;
                to_page.send(Message::text(text)).await?;
                waiting = Some(answer);
            }
            message = from_page.next() => {
                let text = match message {
                    Some(Ok(Message::Text(text))) => text.to_string(),
                    Some(Ok(Message::Close(_))) | None => break Ok(()),
                    Some(Ok(_)) => continue,
                    Some(Err(err)) => break Err(err.into()),
                };
                match pushed(&text, waiting.is_some()) {
                    Some(Pushed::Log(level, line)) => keep_log(&log, &level, &line),
                    Some(Pushed::Hello(..)) => {}
                    Some(Pushed::Other(kind)) => println!("{name} pushed {kind}"),
                    None => match waiting.take() {
                        Some(answer) => drop(answer.send(text.into_bytes())),
                        None => println!("{name} sent an answer nobody asked for"),
                    },
                }
            }
        }
    };
    accept.abort();
    println!("{name} {app_id} left");
    result
}

/// Takes the connections of tools on the inspect port of a page.
async fn take_tools(tools: TcpListener, requests: mpsc::Sender<Request>) {
    loop {
        let Ok((mut stream, _)) = tools.accept().await else {
            return;
        };
        let requests = requests.clone();
        spawn(async move {
            if let Err(err) = carry(&mut stream, &requests).await {
                println!("a tool left: {err:#}");
            }
        });
    }
}

/// Carries the requests of 1 tool to the page, 1 at a time like an app
/// takes them.
async fn carry(stream: &mut TcpStream, requests: &mpsc::Sender<Request>) -> Result<()> {
    loop {
        let Ok(len) = stream.read_u32().await else {
            return Ok(());
        };
        if len > MAX_REQUEST_LEN {
            bail!("a request of {len} bytes is over the limit");
        }
        let mut data = vec![0; usize::try_from(len)?];
        stream.read_exact(&mut data).await?;
        // These carry raw bytes after the command, a page has no way to them.
        let text = String::from_utf8_lossy(&data);
        if text.contains("\"HotBytes\"") || text.contains("\"Speed\"") {
            let refusal = to_vec(&AppCommand::Error("a page takes no raw bytes".to_string()))?;
            write_frame(stream, &refusal).await?;
            return Ok(());
        }
        let (answer, answered) = oneshot::channel();
        requests.send((data, answer)).await.context("the page is gone")?;
        let reply = answered.await.context("the page is gone")?;
        write_frame(stream, &reply).await?;
    }
}

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    stream.write_u32(u32::try_from(data.len())?).await?;
    stream.write_all(data).await?;
    Ok(())
}

fn log_path(app_id: &str) -> PathBuf {
    temp_dir().join(format!("hilen-inspect-page-{app_id}.log"))
}

/// Prints a log line of the page and adds it to its file.
fn keep_log(path: &PathBuf, level: &str, line: &str) {
    println!("{level} {line}");
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| writeln!(file, "{level} {line}"));
    if let Err(err) = written {
        println!("the log of the page is not kept in {}: {err}", path.display());
    }
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;

    use anyhow::Result;
    use futures_util::{SinkExt, StreamExt};
    use hilen::inspect::protocol::{AppCommand, Client, InspectorCommand, discover};
    use serde_json::{from_str, to_string};
    use tokio::{net::TcpListener, spawn};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    use super::{Pushed, pushed, serve_page};

    #[test]
    fn a_frame_of_the_page_is_told_from_an_answer() {
        let log = r#"{"Log":{"level":"INFO","message":"hi"}}"#;
        assert!(
            matches!(pushed(log, true), Some(Pushed::Log(level, line)) if level == "INFO" && line == "hi")
        );
        let report = r#"{"TestResults":{"total":1,"failures":[]}}"#;
        assert!(matches!(pushed(report, false), Some(Pushed::Other(_))));
        assert!(pushed(report, true).is_none());
        assert!(pushed(r#""Ok""#, true).is_none());
        assert!(pushed(r#"{"StartTime":5}"#, true).is_none());
    }

    // A page says hello, a tool then reaches it on the port the bridge
    // opened for it, like an app, and gets the answer of the page.
    #[tokio::test]
    async fn a_tool_reaches_a_page_through_the_bridge() -> Result<()> {
        let pages = TcpListener::bind("127.0.0.1:0").await?;
        let at = pages.local_addr()?;
        spawn(async move {
            let (stream, _) = pages.accept().await.unwrap();
            serve_page(stream).await.unwrap();
        });

        let (mut page, _) = connect_async(format!("ws://{at}/hilen-inspect")).await?;
        let app_id = format!("B{}", std::process::id());
        let hello = AppCommand::Hello {
            app_id: app_id.clone(),
            name:   "bridge-test".to_string(),
        };
        page.send(Message::text(to_string(&hello)?)).await?;

        // The bridge announces the page a moment after its hello, and a
        // search ends soon after the first app it hears.
        let mut port = None;
        for _ in 0..5 {
            port = discover().await?.get(&app_id).map(SocketAddr::port);
            if port.is_some() {
                break;
            }
        }
        let port = port.expect("the page is announced");
        let tool = spawn(async move {
            let client = Client::connect(([127, 0, 0, 1], port).into()).await?;
            client.send(InspectorCommand::GetStartTime).await
        });

        let Some(Ok(Message::Text(asked))) = page.next().await else {
            panic!("the page got no request");
        };
        assert!(matches!(from_str(&asked)?, InspectorCommand::GetStartTime));
        page.send(Message::text(to_string(&AppCommand::StartTime(42))?)).await?;

        assert!(matches!(tool.await??, AppCommand::StartTime(42)));
        Ok(())
    }
}
