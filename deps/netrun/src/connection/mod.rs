const BUFFER_SIZE: usize = 1024 * 16;

mod client;
mod server;
mod service;

pub use client::*;
pub use server::*;
pub use service::*;

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use tokio::{sync::OnceCell, task::JoinSet};

    use super::*;
    use crate::Retry;

    async fn server() -> Result<&'static Server<i32, bool>> {
        static SERVER: OnceCell<Server<i32, bool>> = OnceCell::const_new();

        SERVER
            .get_or_try_init(|| async {
                let s = Server::start(57777).await?;
                Ok(s)
            })
            .await
    }

    #[test(tokio::test)]
    async fn test_static_server() -> Result<()> {
        assert!(server().await.is_ok());
        let client = Client::<bool, i32>::connect((Ipv4Addr::LOCALHOST, 57777)).await?;
        let connection = server().await?.wait_for_new_connection().await;

        client.send(5).await?;
        assert_eq!(5, connection.receive().await?);

        connection.send(true).await?;
        assert!(client.receive().await?);

        drop(client);

        Ok(())
    }

    #[test(tokio::test)]
    async fn test_mismatched_types() -> Result<()> {
        let server = Server::<i32, i32>::start(57778).await?;
        let client = Client::<bool, i32>::connect((Ipv4Addr::LOCALHOST, 57778)).await?;
        let connection = server.wait_for_new_connection().await;

        client.send(5).await?;
        assert_eq!(5, connection.receive().await?);

        connection.send(true).await?;

        let err = client.receive().await.err().unwrap();

        assert_eq!(
            r"Failed to deserialize from client: invalid type: integer `1`, expected a boolean at line 1 column 1",
            err.to_string()
        );

        Ok(())
    }

    async fn test_connection(port: u16) -> Result<()> {
        let server = Server::<i32, bool>::start(port).await?;

        let client = Client::<bool, i32>::connect((Ipv4Addr::LOCALHOST, port)).await?;
        let connection = server.wait_for_new_connection().await;

        client.send(5).await?;
        assert_eq!(5, connection.receive().await?);

        connection.send(true).await?;
        assert!(client.receive().await?);

        drop(client);

        Ok(())
    }

    #[test(tokio::test)]
    async fn stress_test_connection() -> Result<()> {
        let mut set = JoinSet::new();

        for i in 64000..64100 {
            set.spawn(async move { test_connection(i).await });
        }

        while let Some(res) = set.join_next().await {
            let output = res.expect("Task panicked");
            println!("{:?}", output?);
        }

        Ok(())
    }

    #[test(tokio::test)]
    async fn server_without_connections() -> Result<()> {
        let server = Server::<i32, i32>::start(55432).await?;

        let result = Retry::times(1)
            .timeout(200)
            .run(|| async {
                server.wait_for_new_connection().await;
                Ok(())
            })
            .await;

        assert_eq!("Retry exceeded", result.err().unwrap().to_string());

        Ok(())
    }

    #[test(tokio::test)]
    async fn connection_debug_impl() -> Result<()> {
        let server = Server::<i32, bool>::start(55550).await?;
        let client = Client::<bool, i32>::connect((Ipv4Addr::LOCALHOST, 55550)).await?;

        assert_eq!("Server<i32, bool> { port: 55550 }", format!("{server:?}"));

        let client_dbg = format!("{client:?}");

        assert!(
            client_dbg.contains("address: 127.0.0.1:55550 }") && client_dbg.contains("Client<bool, i32>")
        );

        Ok(())
    }
}
