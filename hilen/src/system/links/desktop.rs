//! A desktop app gets its link as an argument. The packaging registers the
//! url scheme of the app with the system, see the `[links]` table of
//! `hilen.toml`, and a click on such a link starts the app with the link
//! after the program name. On macOS a bundle gets it as an Apple Event, see
//! `macos.rs`, an argument works there too, for a run from the terminal.
//!
//! A second start with a link hands the link to the app that runs already
//! and quits. The running app listens on a TCP port of `127.0.0.1` and
//! keeps the number in `links.port` in its data folder. A start reads the
//! number, sends the link and waits for the answer of the same app. With no
//! answer it is the first app and listens itself.

use std::{
    env::args,
    fs::{read_to_string, write},
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::exit,
    thread::Builder,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use log::{debug, info, warn};

use crate::{app::hilen_project_name, deps::hreads::on_main, filesystem::Paths, window::ready_winit_window};

const PORT_FILE: &str = "links.port";

/// The first word of a request and of its answer, so a port that another
/// program took after a crash is not taken for the app.
const HELLO: &str = "hilen-link";

/// How long a start waits for the running app, and the running app for the
/// lines of a request.
const WAIT: Duration = Duration::from_secs(2);

/// Runs at the start of a desktop app with a window, before `before_launch`.
/// A start with a link that the running app took ends the process here.
pub(crate) fn start() {
    #[cfg(macos)]
    super::macos::install();

    let Some(scheme) = crate::app::hilen_link_scheme() else {
        debug!("no [links] table in hilen.toml, the app takes no link at its start");
        return;
    };
    let name = hilen_project_name();
    let port_file = Paths::storage().join(PORT_FILE);
    let link = link_in_args(args().skip(1), scheme);

    if let Some(link) = &link {
        match hand_over(&port_file, name, link) {
            Ok(()) => {
                info!("the running {name} took the link, this start ends");
                exit(0);
            }
            Err(err) => debug!("no running {name} took the link: {err:#}"),
        }
    }

    match listen(&port_file, name, |url| {
        super::receive(url, "a second start");
        on_main(|| {
            if let Some(window) = ready_winit_window() {
                window.focus_window();
            }
        });
    }) {
        Ok(port) => debug!("links of a second start come on port {port}"),
        Err(err) => warn!("a second start cannot hand its link to this app: {err:#}"),
    }

    if let Some(link) = link {
        super::receive(link, "the start arguments");
    }
}

/// The first argument that is a link of `scheme`.
fn link_in_args(args: impl IntoIterator<Item = String>, scheme: &str) -> Option<String> {
    let prefix = format!("{scheme}:");
    args.into_iter()
        .find(|arg| arg.get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(&prefix)))
}

/// Sends `link` to the app that wrote `port_file`. `Ok` only when that app
/// answered that it took it.
fn hand_over(port_file: &Path, name: &str, link: &str) -> Result<()> {
    let port: u16 = read_to_string(port_file)
        .with_context(|| format!("no {}", port_file.display()))?
        .trim()
        .parse()
        .with_context(|| format!("no port number in {}", port_file.display()))?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&address, WAIT).with_context(|| format!("port {port}"))?;
    stream.set_read_timeout(Some(WAIT))?;
    stream.write_all(format!("{HELLO} {name}\n{link}\n").as_bytes())?;

    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer)?;
    ensure!(
        answer.trim_end() == format!("{HELLO} {name} ok"),
        "port {port} gave another answer: {answer:?}"
    );
    Ok(())
}

/// Listens for the links of later starts on a free port and writes its
/// number into `port_file`. `take` gets every link, on the thread of the
/// listener.
fn listen(port_file: &Path, name: &'static str, take: impl Fn(String) + Send + 'static) -> Result<u16> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    write(port_file, port.to_string()).with_context(|| format!("cannot write {}", port_file.display()))?;

    let port_file = PathBuf::from(port_file);
    Builder::new().name("links".into()).spawn(move || {
        for stream in listener.incoming() {
            let request = stream.map_err(anyhow::Error::from).and_then(|stream| answer(stream, name));
            match request {
                Ok(link) => take(link),
                Err(err) => warn!(
                    "a wrong request on the links port of {}: {err:#}",
                    port_file.display()
                ),
            }
        }
    })?;
    Ok(port)
}

/// Reads 1 request and answers it, the link of the request.
fn answer(stream: TcpStream, name: &str) -> Result<String> {
    stream.set_read_timeout(Some(WAIT))?;
    let mut reader = BufReader::new(stream);
    let mut hello = String::new();
    reader.read_line(&mut hello)?;
    ensure!(
        hello.trim_end() == format!("{HELLO} {name}"),
        "it is not for this app: {hello:?}"
    );
    let mut link = String::new();
    reader.read_line(&mut link)?;
    let link = link.trim_end().to_string();
    ensure!(!link.is_empty(), "it has no link");

    reader.into_inner().write_all(format!("{HELLO} {name} ok\n").as_bytes())?;
    Ok(link)
}

#[cfg(test)]
mod test {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_dir_all, write},
        io::Read,
        net::{Ipv4Addr, TcpListener},
        path::PathBuf,
        process::id,
        sync::mpsc::channel,
        thread::spawn,
        time::{Duration, Instant},
    };

    use anyhow::Result;

    use super::{PORT_FILE, hand_over, link_in_args, listen};

    struct Folder(PathBuf);

    impl Folder {
        fn new(test: &str) -> Result<Self> {
            let dir = temp_dir().join(format!("hilen-links-{test}-{}", id()));
            create_dir_all(&dir)?;
            Ok(Self(dir))
        }

        fn port_file(&self) -> PathBuf {
            self.0.join(PORT_FILE)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.0);
        }
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_link_is_the_argument_with_the_scheme_of_the_app() {
        assert_eq!(
            link_in_args(args(&["--flag", "shopping://invite/x7k2"]), "shopping").as_deref(),
            Some("shopping://invite/x7k2")
        );
        assert_eq!(
            link_in_args(args(&["Shopping:invite/x7k2"]), "shopping").as_deref(),
            Some("Shopping:invite/x7k2")
        );
        assert_eq!(
            link_in_args(args(&["https://shopping.vladas.xyz/invite/x"]), "shopping"),
            None
        );
        assert_eq!(link_in_args(args(&["shoppingx://invite"]), "shopping"), None);
        assert_eq!(link_in_args(args(&["shop"]), "shopping"), None);
        assert_eq!(link_in_args(args(&[]), "shopping"), None);
    }

    #[test]
    fn a_second_start_hands_its_link_to_the_running_app() -> Result<()> {
        let folder = Folder::new("hand-over")?;
        let (sender, taken) = channel();
        listen(&folder.port_file(), "shopping", move |link| {
            sender.send(link).unwrap();
        })?;

        hand_over(&folder.port_file(), "shopping", "shopping://invite/x7k2")?;
        hand_over(&folder.port_file(), "shopping", "shopping://invite/second")?;

        let wait = Duration::from_secs(5);
        assert_eq!(taken.recv_timeout(wait)?, "shopping://invite/x7k2");
        assert_eq!(taken.recv_timeout(wait)?, "shopping://invite/second");
        Ok(())
    }

    #[test]
    fn another_app_does_not_take_the_link() -> Result<()> {
        let folder = Folder::new("other-app")?;
        let (sender, taken) = channel();
        listen(&folder.port_file(), "banda", move |link| {
            sender.send(link).unwrap();
        })?;

        assert!(hand_over(&folder.port_file(), "shopping", "shopping://invite/x").is_err());
        assert!(taken.recv_timeout(Duration::from_millis(300)).is_err());
        Ok(())
    }

    #[test]
    fn with_no_running_app_the_start_keeps_its_link() -> Result<()> {
        let folder = Folder::new("none")?;
        assert!(hand_over(&folder.port_file(), "shopping", "shopping://x").is_err());

        // A port that nobody listens on any more, from an app that crashed.
        let gone = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?.local_addr()?.port();
        write(folder.port_file(), gone.to_string())?;
        assert!(hand_over(&folder.port_file(), "shopping", "shopping://x").is_err());
        Ok(())
    }

    /// A program that took the port after a crash and never answers. The
    /// start gives up after its wait and opens as the first app.
    #[test]
    fn a_port_that_never_answers_is_not_the_app() -> Result<()> {
        let folder = Folder::new("silent")?;
        let silent = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        write(folder.port_file(), silent.local_addr()?.port().to_string())?;
        spawn(move || {
            let (mut stream, _) = silent.accept().unwrap();
            let mut buffer = Vec::new();
            let _ = stream.read_to_end(&mut buffer);
        });

        let started = Instant::now();
        assert!(hand_over(&folder.port_file(), "shopping", "shopping://x").is_err());
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "{:?}",
            started.elapsed()
        );
        Ok(())
    }
}
