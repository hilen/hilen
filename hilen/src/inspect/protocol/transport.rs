use std::{
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use anyhow::{Result, bail};
use if_addrs::{IfAddr, get_if_addrs};
use log::debug;
use serde_json::{from_slice, to_vec};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
    spawn,
    sync::Mutex,
};

use crate::{
    deps::hreads::on_main,
    inspect::protocol::{AppCommand, InspectorCommand},
};

/// A request is a small command. The length prefix comes from whoever
/// connects, a bigger one is refused before the buffer is allocated.
const MAX_REQUEST_LEN: u32 = 64 * 1024 * 1024;

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
        let response = handler(from_slice(&request)?);
        let data = to_vec(&response);

        // The response can hold Own pointers, which must drop on the main
        // thread.
        on_main(move || drop(response));

        write_frame(stream, &data?).await?;
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
    use std::net::Ipv4Addr;

    use super::{MAX_REQUEST_LEN, own_address_near, read_frame};

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
