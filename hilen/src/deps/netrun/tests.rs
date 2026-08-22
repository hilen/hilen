#![cfg(test)]

use anyhow::Result;

#[test]
fn test_local_ip() -> Result<()> {
    dbg!(crate::deps::netrun::local_ip()?);

    Ok(())
}
