use anyhow::{Result, bail};
use log::debug;
use serde_json::{from_slice, to_vec};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
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
    pub async fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            stream: Mutex::new(TcpStream::connect(addr).await?),
        })
    }

    pub async fn send(&self, command: InspectorCommand) -> Result<AppCommand> {
        let mut stream = self.stream.lock().await;
        write_frame(&mut stream, &to_vec(&command)?).await?;
        // The reply comes from the app the client chose, a screenshot can be
        // large.
        Ok(from_slice(&read_frame(&mut *stream, u32::MAX).await?)?)
    }
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
    use super::{MAX_REQUEST_LEN, read_frame};

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
