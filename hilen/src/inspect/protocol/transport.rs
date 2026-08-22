use anyhow::Result;
use log::debug;
use serde_json::{from_slice, to_vec};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    spawn,
    sync::Mutex,
};

use crate::{
    deps::hreads::on_main,
    inspect::protocol::{AppCommand, InspectorCommand},
};

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
        Ok(from_slice(&read_frame(&mut stream).await?)?)
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
        let request = read_frame(stream).await?;
        let response = handler(from_slice(&request)?);
        let data = to_vec(&response);

        // The response can hold Own pointers, which must drop on the main thread.
        on_main(move || drop(response));

        write_frame(stream, &data?).await?;
    }
}

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    stream.write_u32(u32::try_from(data.len())?).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let len = stream.read_u32().await?;
    let mut data = vec![0; usize::try_from(len)?];
    stream.read_exact(&mut data).await?;
    Ok(data)
}
