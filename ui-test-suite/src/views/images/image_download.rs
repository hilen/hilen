use std::{
    env::temp_dir,
    fs::{read_dir, remove_dir_all},
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Mutex, mpsc::channel},
    thread::spawn as spawn_thread,
    time::Duration,
};

use anyhow::{Result, anyhow, ensure};
use hilen::{
    dispatch::{from_main, spawn, wait_for_next_frame},
    refs::{Weak, manage::DataManager},
    ui::{ImageView, Label, Setup, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
    window::image::Image,
};
use log::debug;

/// What the server hands out for every path, 550 by 775 pixels.
const PICTURE: &[u8] = include_bytes!("../../../../assets/images/cat.png");
/// The bytes of pixels one such picture holds once it is decoded.
const PICTURE_BYTES: usize = 550 * 775 * 4;

const HEADER: &str = "X-Hilen-Token";
const TOKEN: &str = "secret-of-the-test";

/// The path of every request the server answered with the picture.
static SERVED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Serves the picture on a free local port, only to a request that carries
/// the token header. Returns the address without a path.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port is free");
    let port = listener.local_addr().expect("a bound listener has an address").port();
    spawn_thread(move || {
        for stream in listener.incoming().flatten() {
            spawn_thread(move || {
                if let Err(err) = respond(stream) {
                    debug!("the test image server stopped a response: {err}");
                }
            });
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn respond(mut stream: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut head = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        head.push(line.trim().to_lowercase());
    }
    let path = head
        .first()
        .and_then(|line| line.split(' ').nth(1))
        .unwrap_or_default()
        .to_string();
    let allowed = head.contains(&format!("{}: {TOKEN}", HEADER.to_lowercase()));

    if allowed {
        SERVED.lock().expect("the request log is not poisoned").push(path);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            PICTURE.len()
        )?;
        stream.write_all(PICTURE)?;
    } else {
        write!(
            stream,
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )?;
    }
    Ok(())
}

/// Downloads on the async runtime and waits for the answer.
fn download(name: &str, url: String, token: bool) -> Result<Weak<Image>> {
    let name = name.to_string();
    let (send, receive) = channel();
    spawn(async move {
        let headers: &[(&str, &str)] = if token { &[(HEADER, TOKEN)] } else { &[] };
        let image = Image::download_with(&name, &url, headers).await.map_err(|err| err.to_string());
        if send.send(image).is_err() {
            debug!("the test stopped waiting for {name}");
        }
    });
    receive.recv_timeout(Duration::from_secs(10))?.map_err(|err| anyhow!(err))
}

fn served(path: &str) -> usize {
    SERVED
        .lock()
        .expect("the request log is not poisoned")
        .iter()
        .filter(|seen| *seen == path)
        .count()
}

fn alive(name: &'static str) -> bool {
    from_main(move || Image::get_existing(name).is_some())
}

/// Puts the image store back the way other tests expect it.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        from_main(|| {
            Image::set_download_memory_limit(0);
            Image::set_download_cache_dir(None);
            for name in ["poster-1", "poster-2", "poster-3"] {
                Image::free_with_name(name);
            }
        });
    }
}

/// Posters from a server behind a token. Proves a download with no token is
/// refused and one with the header gets the picture, a second download of
/// the same url after the image was freed comes from the disk cache with no
/// request, and the memory bound frees the image drawn longest ago and never
/// the one on screen.
#[view]
struct ImageDownload {
    #[init]
    title:  Label,
    poster: ImageView,
    state:  Label,
}

impl Setup for ImageDownload {
    fn setup(self: Weak<Self>) {
        self.title.set_frame((20, 20, 560, 40));
        self.title.set_text("posters from a server behind a token");
        self.poster.set_frame((200, 80, 200, 282));
        self.state.set_frame((20, 380, 560, 40));
        self.state.set_text("nothing downloaded");
    }
}

impl ViewTest for ImageDownload {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        SERVED.lock().expect("the request log is not poisoned").clear();
        let server = serve();
        let cache = temp_dir().join(format!("hilen-image-cache-{}", std::process::id()));
        let cache_for_main = cache.clone();
        from_main(move || Image::set_download_cache_dir(Some(cache_for_main)));
        let restore = Restore;

        let show = move |image: Weak<Image>, text: &'static str| {
            from_main(move || {
                view.poster.set_image(image);
                view.state.set_text(text);
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };

        ensure!(
            download("poster-1", format!("{server}/1"), false).is_err(),
            "a request with no token is refused"
        );
        let first = download("poster-1", format!("{server}/1"), true)?;
        ensure!(served("/1") == 1, "the token header reached the server");
        show(first, "poster 1, from the server");
        checkpoint("poster 1 shows, fetched with the token header")?;

        // Freed and asked again: the bytes come from disk, not the server.
        from_main(|| Image::free_with_name("poster-1"));
        ensure!(!alive("poster-1"), "the image is freed");
        let again = download("poster-1", format!("{server}/1"), true)?;
        ensure!(served("/1") == 1, "the second download made no request");
        ensure!(
            read_dir(&cache)?.count() == 1,
            "the cache folder holds the one picture"
        );
        show(again, "poster 1 again, from the disk cache");
        checkpoint("poster 1 shows again, with no request")?;

        // Room for 2 pictures. Poster 2 is never drawn, poster 3 goes on
        // screen, so the third download pushes poster 2 out.
        from_main(|| Image::set_download_memory_limit(PICTURE_BYTES * 2 + 1));
        download("poster-2", format!("{server}/2"), true)?;
        let third = download("poster-3", format!("{server}/3"), true)?;
        show(third, "poster 3, the bound freed poster 2");
        ensure!(!alive("poster-2"), "the image never drawn is freed first");
        ensure!(alive("poster-1") && alive("poster-3"), "2 pictures fit the bound");
        checkpoint("poster 3 shows, poster 2 is gone")?;

        // No room at all: what was drawn before goes, what is on screen stays.
        from_main(|| Image::set_download_memory_limit(1));
        wait_for_next_frame();
        ensure!(!alive("poster-1"), "the image drawn longest ago is freed");
        ensure!(alive("poster-3"), "the image on screen is never freed");
        checkpoint("poster 3 still shows, the rest is freed")?;

        drop(restore);
        remove_dir_all(&cache)?;
        Ok(())
    }
}
