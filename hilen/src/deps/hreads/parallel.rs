#![cfg(not_wasm)]

use std::sync::Arc;

use log::error;
use tokio::{
    select, spawn,
    sync::{Mutex, mpsc::channel},
};

pub async fn first_ok<F, Output, E>(futures: impl IntoIterator<Item = F>) -> Result<Output, E>
where
    Output: Send + 'static,
    E: Send + 'static,
    F: Future<Output = Result<Output, E>> + Send + 'static, {
    let counter = Arc::new(Mutex::new(0));
    let (ok_sender, mut ok_receiver) = channel::<Output>(1);
    let (err_sender, mut err_receiver) = channel::<E>(1);

    let futures: Vec<_> = futures.into_iter().collect();
    let len = futures.len();

    for fut in futures {
        let ok_sender = ok_sender.clone();
        let err_sender = err_sender.clone();
        let counter = counter.clone();
        spawn(async move {
            let result = fut.await;

            match result {
                Ok(result) => {
                    _ = ok_sender
                        .send(result)
                        .await
                        .inspect_err(|e| error!("Failed to send ok result: {e}"));
                }
                Err(err) => {
                    let mut counter = counter.lock().await;
                    *counter += 1;

                    if *counter == len {
                        _ = err_sender.send(err).await;
                    }
                }
            }
        });
    }

    select! {
        ok = ok_receiver.recv() => Ok(ok.unwrap()),
        err = err_receiver.recv() => Err(err.unwrap()),
    }
}

#[cfg(test)]
mod test {
    use anyhow::{Result, anyhow, bail};
    use fake::{Fake, Faker};
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn all_ok() -> Result<()> {
        let result = first_ok((0..5).map(|_| async move { Ok::<i32, anyhow::Error>(55) })).await?;

        assert_eq!(55, result);

        Ok(())
    }

    #[tokio::test]
    async fn some_ok() -> Result<()> {
        let result = first_ok((0..50).map(|_| async move {
            if Faker.fake::<bool>() {
                Ok(77)
            } else {
                bail!("allal")
            }
        }))
        .await?;

        assert_eq!(77, result);

        Ok(())
    }

    #[tokio::test]
    async fn all_err() -> Result<()> {
        let result: Result<i32, _> = first_ok((0..50).map(|_| async move { bail!("allal") })).await;

        assert_eq!(anyhow!("allal").to_string(), result.err().unwrap().to_string());

        Ok(())
    }
}
