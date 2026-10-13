use std::{
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::{Result, bail};
use if_addrs::{IfAddr, get_if_addrs};
use log::debug;
use serde_json::{from_slice, to_vec};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
    spawn,
    sync::Mutex,
    time::Instant,
};

use crate::{
    deps::hreads::on_main,
    inspect::protocol::{AppCommand, InspectorCommand},
};

/// A request is a small command. The length prefix comes from whoever
/// connects, a bigger one is refused before the buffer is allocated.
const MAX_REQUEST_LEN: u32 = 64 * 1024 * 1024;
/// A file for a hot build goes in raw pieces of this size.
const FILE_PIECE: usize = 4 * 1024 * 1024;
/// The bytes of a speed test go in pieces of this size, each behind its
/// length.
const SPEED_PIECE: usize = 256 * 1024;
/// An app sends no longer than this, whatever the client asks for.
const MAX_SPEED: Duration = Duration::from_secs(30);

/// What 1 run of the speed test carried, as the receiving side counted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedRun {
    pub bytes: u64,
    pub time:  Duration,
}

impl SpeedRun {
    pub fn mb_per_second(&self) -> f64 {
        megabytes(self.bytes) / self.time.as_secs_f64()
    }
}

pub fn megabytes(bytes: u64) -> f64 {
    f64::from(u32::try_from(bytes / 1024).unwrap_or(u32::MAX)) / 1024.0
}

pub struct Client {
    stream: Mutex<TcpStream>,
}

impl Client {
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let IpAddr::V4(target) = addr.ip() else {
            return Ok(Self {
                stream: Mutex::new(TcpStream::connect(addr).await?),
            });
        };

        let socket = TcpSocket::new_v4()?;
        // A VPN can hold a route for the whole local network and take the
        // packets for an app on a phone. A socket that starts from the
        // address this machine has in that network goes out there.
        if let Some(own) = own_address_near(target, &local_networks()?) {
            socket.bind(SocketAddr::new(own.into(), 0))?;
        }
        Ok(Self {
            stream: Mutex::new(socket.connect(addr).await?),
        })
    }

    pub async fn send(&self, command: InspectorCommand) -> Result<AppCommand> {
        let mut stream = self.stream.lock().await;
        write_frame(&mut stream, &to_vec(&command)?).await?;
        // The reply comes from the app the client chose, a screenshot can be
        // large.
        Ok(from_slice(&read_frame(&mut *stream, u32::MAX).await?)?)
    }

    /// Sends a file to a hot build in raw pieces, `HotBytes`. Every piece
    /// goes out before the first answer is read, the answers are a few
    /// bytes each and wait in the buffers.
    pub async fn send_file(&self, file: &str, data: &[u8]) -> Result<()> {
        let mut stream = self.stream.lock().await;
        // An empty file has no piece, it is still made.
        let pieces: Vec<&[u8]> = if data.is_empty() {
            vec![data]
        } else {
            data.chunks(FILE_PIECE).collect()
        };
        let mut offset = 0;
        for piece in &pieces {
            let len = u32::try_from(piece.len())?;
            let command = InspectorCommand::HotBytes {
                file: file.to_string(),
                offset,
                len,
                data: Vec::new(),
            };
            write_frame(&mut stream, &to_vec(&command)?).await?;
            stream.write_all(piece).await?;
            offset += u64::from(len);
        }
        for _ in &pieces {
            if let AppCommand::Error(err) = from_slice(&read_frame(&mut *stream, MAX_REQUEST_LEN).await?)? {
                bail!("{err}");
            }
        }
        Ok(())
    }

    /// Sends bytes to the app for `time`. `each` gets the bytes sent so
    /// far after every piece. The run ends when the app said how many it
    /// got, so bytes that still wait in a buffer are not counted as fast.
    pub async fn speed_to_app(&self, time: Duration, each: impl FnMut(u64)) -> Result<SpeedRun> {
        let mut stream = self.stream.lock().await;
        let command = InspectorCommand::Speed {
            to_app: true,
            ms:     0,
        };
        write_frame(&mut stream, &to_vec(&command)?).await?;
        let started = Instant::now();
        pour(&mut *stream, time, each).await?;
        let bytes = speed_answer(&mut stream).await?;
        Ok(SpeedRun {
            bytes,
            time: started.elapsed(),
        })
    }

    /// Makes the app send bytes for `time` and counts them here. `each`
    /// gets the bytes that came so far after every piece.
    pub async fn speed_from_app(&self, time: Duration, each: impl FnMut(u64)) -> Result<SpeedRun> {
        let mut stream = self.stream.lock().await;
        let command = InspectorCommand::Speed {
            to_app: false,
            ms:     u32::try_from(time.as_millis())?,
        };
        write_frame(&mut stream, &to_vec(&command)?).await?;
        let started = Instant::now();
        let bytes = drain(&mut *stream, each).await?;
        let time = started.elapsed();
        speed_answer(&mut stream).await?;
        Ok(SpeedRun { bytes, time })
    }

    /// Returns once the app closes the connection, which a quitting app
    /// does only when its process ends.
    pub async fn closed(&self) -> Result<()> {
        let mut stream = self.stream.lock().await;
        let mut byte = [0];
        match stream.read(&mut byte).await {
            Ok(0) => Ok(()),
            Ok(_) => bail!("App sent data after the quit reply"),
            Err(err) if err.kind() == ErrorKind::ConnectionReset => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

/// The address and the mask of every network this machine is in.
fn local_networks() -> Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
    let mut networks = vec![];
    for interface in get_if_addrs()? {
        if let IfAddr::V4(v4) = interface.addr {
            networks.push((v4.ip, v4.netmask));
        }
    }
    Ok(networks)
}

/// The address of this machine in the network that `target` is in. None for
/// an app on this machine and for a target behind a router.
fn own_address_near(target: Ipv4Addr, networks: &[(Ipv4Addr, Ipv4Addr)]) -> Option<Ipv4Addr> {
    if target.is_loopback() {
        return None;
    }
    let target = target.to_bits();
    networks
        .iter()
        .find(|(own, mask)| !own.is_loopback() && own.to_bits() & mask.to_bits() == target & mask.to_bits())
        .map(|(own, _)| *own)
}

pub(crate) async fn serve(listener: TcpListener, handler: fn(InspectorCommand) -> AppCommand) -> Result<()> {
    loop {
        let (mut stream, addr) = listener.accept().await?;
        spawn(async move {
            if let Err(err) = handle_connection(&mut stream, handler).await {
                debug!("Inspector connection {addr} closed: {err}");
            }
        });
    }
}

async fn handle_connection(
    stream: &mut TcpStream,
    handler: fn(InspectorCommand) -> AppCommand,
) -> Result<()> {
    loop {
        let request = read_frame(stream, MAX_REQUEST_LEN).await?;
        let mut command = from_slice(&request)?;
        if let InspectorCommand::HotBytes { len, data, .. } = &mut command {
            if *len > MAX_REQUEST_LEN {
                bail!("A piece of {len} bytes is over the limit of {MAX_REQUEST_LEN}");
            }
            *data = vec![0; usize::try_from(*len)?];
            stream.read_exact(data).await?;
        }
        // The bytes of a speed test follow on this connection as they are,
        // the handler has no way to them.
        if let InspectorCommand::Speed { to_app, ms } = command {
            let answer = speed(stream, to_app, ms).await?;
            write_frame(stream, &to_vec(&answer)?).await?;
            continue;
        }
        let response = handler(command);
        let data = to_vec(&response);

        // The response can hold Own pointers, which must drop on the main
        // thread.
        on_main(move || drop(response));

        write_frame(stream, &data?).await?;
    }
}

/// The app side of a run of the speed test.
async fn speed(stream: &mut TcpStream, to_app: bool, ms: u32) -> Result<AppCommand> {
    let started = Instant::now();
    let bytes = if to_app {
        drain(stream, |_| ()).await?
    } else {
        let time = Duration::from_millis(ms.into()).min(MAX_SPEED);
        pour(stream, time, |_| ()).await?
    };
    Ok(AppCommand::Speed {
        bytes,
        ms: u64::try_from(started.elapsed().as_millis())?,
    })
}

async fn speed_answer(stream: &mut TcpStream) -> Result<u64> {
    match from_slice(&read_frame(stream, MAX_REQUEST_LEN).await?)? {
        AppCommand::Speed { bytes, .. } => Ok(bytes),
        AppCommand::Error(err) => bail!("{err}"),
        _ => bail!("Unexpected response to the speed test"),
    }
}

/// Bytes with no pattern, so nothing on the way can pack them and make a
/// link look faster than it is.
fn filler() -> Vec<u8> {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    (0..SPEED_PIECE)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect()
}

/// Writes pieces for `time`, then the length 0 that ends the run. Gives
/// the bytes of the pieces.
async fn pour(
    stream: &mut (impl AsyncWrite + Unpin),
    time: Duration,
    mut each: impl FnMut(u64),
) -> Result<u64> {
    let piece = filler();
    let len = u32::try_from(piece.len())?;
    let started = Instant::now();
    let mut total = 0;
    while started.elapsed() < time {
        stream.write_u32(len).await?;
        stream.write_all(&piece).await?;
        total += u64::from(len);
        each(total);
    }
    stream.write_u32(0).await?;
    stream.flush().await?;
    Ok(total)
}

/// Reads pieces up to the length 0 and throws them away. Gives their bytes.
async fn drain(stream: &mut (impl AsyncRead + Unpin), mut each: impl FnMut(u64)) -> Result<u64> {
    let mut piece = vec![0; SPEED_PIECE];
    let mut total = 0;
    loop {
        let len = usize::try_from(stream.read_u32().await?)?;
        if len == 0 {
            return Ok(total);
        }
        if len > SPEED_PIECE {
            bail!("A piece of {len} bytes is over the limit of {SPEED_PIECE}");
        }
        stream.read_exact(&mut piece[..len]).await?;
        total += u64::try_from(len)?;
        each(total);
    }
}

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    stream.write_u32(u32::try_from(data.len())?).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn read_frame(stream: &mut (impl AsyncRead + Unpin), max_len: u32) -> Result<Vec<u8>> {
    let len = stream.read_u32().await?;
    if len > max_len {
        bail!("Frame of {len} bytes is over the limit of {max_len}");
    }
    let mut data = vec![0; usize::try_from(len)?];
    stream.read_exact(&mut data).await?;
    Ok(data)
}

#[cfg(test)]
mod test {
    use std::{
        net::Ipv4Addr,
        sync::{Mutex, PoisonError},
        time::Duration,
    };

    use anyhow::Result;
    use tokio::{io::duplex, join, net::TcpListener, spawn};

    use super::{
        Client, FILE_PIECE, MAX_REQUEST_LEN, SPEED_PIECE, SpeedRun, drain, filler, own_address_near, pour,
        read_frame, serve,
    };
    use crate::{
        deps::hreads::set_current_thread_as_main,
        inspect::protocol::{AppCommand, InspectorCommand},
    };

    #[tokio::test]
    async fn a_speed_run_counts_every_byte_it_carried() -> Result<()> {
        let (mut near, mut far) = duplex(SPEED_PIECE);
        let (sent, got) = join!(
            pour(&mut near, Duration::from_millis(50), |_| ()),
            drain(&mut far, |_| ())
        );
        let got = got?;
        assert_eq!(sent?, got);
        assert!(got >= u64::try_from(SPEED_PIECE)?);
        Ok(())
    }

    #[tokio::test]
    async fn a_client_measures_both_directions_against_a_served_app() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server = spawn(serve(listener, |_| AppCommand::Ok));
        let client = Client::connect(addr).await?;
        let time = Duration::from_millis(300);

        let up = client.speed_to_app(time, |_| ()).await?;
        let down = client.speed_from_app(time, |_| ()).await?;
        // The connection is in step again after a run, it takes the next one.
        let again = client.speed_to_app(time, |_| ()).await?;
        assert!(again.bytes >= u64::try_from(SPEED_PIECE)?);

        assert!(up.bytes >= u64::try_from(SPEED_PIECE)?);
        assert!(down.bytes >= u64::try_from(SPEED_PIECE)?);
        server.abort();
        Ok(())
    }

    static GOT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    fn keep(command: InspectorCommand) -> AppCommand {
        let InspectorCommand::HotBytes { offset, data, .. } = command else {
            return AppCommand::Error("not a piece".into());
        };
        let mut got = GOT.lock().unwrap_or_else(PoisonError::into_inner);
        if u64::try_from(got.len()).ok() != Some(offset) {
            return AppCommand::Error("a piece out of order".into());
        }
        got.extend(data);
        AppCommand::Ok
    }

    #[tokio::test]
    async fn a_file_arrives_whole_from_raw_pieces() -> Result<()> {
        set_current_thread_as_main();
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server = spawn(serve(listener, keep));
        let client = Client::connect(addr).await?;
        // 2 whole pieces and a part of a third one.
        let file: Vec<u8> = filler().into_iter().cycle().take(FILE_PIECE * 2 + 5).collect();

        client.send_file("lib.dylib", &file).await?;

        assert!(*GOT.lock().unwrap_or_else(PoisonError::into_inner) == file);
        server.abort();
        Ok(())
    }

    #[tokio::test]
    async fn a_piece_over_the_limit_ends_the_speed_run() {
        let mut stream: &[u8] = &u32::MAX.to_be_bytes();
        let error = drain(&mut stream, |_| ()).await.unwrap_err();
        assert!(error.to_string().contains("over the limit"));
    }

    #[test]
    fn a_speed_run_gives_megabytes_per_second() {
        let run = SpeedRun {
            bytes: 96 * 1024 * 1024,
            time:  Duration::from_secs(2),
        };
        assert!((run.mb_per_second() - 48.0).abs() < 0.001);
    }

    #[test]
    fn a_connect_starts_from_the_address_in_the_network_of_the_app() {
        let networks = [
            (Ipv4Addr::LOCALHOST, Ipv4Addr::new(255, 0, 0, 0)),
            (Ipv4Addr::new(10, 12, 251, 217), Ipv4Addr::new(255, 255, 255, 128)),
            (Ipv4Addr::new(192, 168, 0, 11), Ipv4Addr::new(255, 255, 255, 0)),
        ];

        let phone = Ipv4Addr::new(192, 168, 0, 208);
        assert_eq!(
            own_address_near(phone, &networks),
            Some(Ipv4Addr::new(192, 168, 0, 11))
        );
        // An app on this machine, and one behind a router.
        assert_eq!(own_address_near(Ipv4Addr::LOCALHOST, &networks), None);
        assert_eq!(own_address_near(Ipv4Addr::new(8, 8, 8, 8), &networks), None);
    }

    #[tokio::test]
    async fn oversized_request_is_refused_before_reading() {
        let mut stream: &[u8] = &u32::MAX.to_be_bytes();

        let error = read_frame(&mut stream, MAX_REQUEST_LEN).await.unwrap_err();

        assert!(error.to_string().contains("over the limit"));
    }

    #[tokio::test]
    async fn request_within_limit_is_read() {
        let mut stream: &[u8] = &[0, 0, 0, 2, b'{', b'}'];

        assert_eq!(read_frame(&mut stream, MAX_REQUEST_LEN).await.unwrap(), b"{}");
    }
}
