//! How a finished login goes back to the app that started it.
//!
//! Whoever made the login link holds its verifier. If the poll with the
//! verifier were enough, a person who sends their own link to someone else
//! would get the session of that someone once they log in. So at the end of
//! the login the server gives a one time code to the app instance that
//! started it, on a way that only that instance can read, and the poll needs
//! the code too. The browser of the person who logged in holds the code, the
//! one who made the link does not.

use anyhow::{Result, bail};

/// The way back, chosen by the app in the start link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandBack {
    /// A native app listens on this port of `127.0.0.1` while the login
    /// runs. The browser is sent there with the code, so the code reaches
    /// only a program on the same device as the browser.
    Loopback(u16),
    /// The app is a page of this same server. The last page writes the code
    /// into the `localStorage` of this origin, where only pages of this
    /// origin in the same browser read it.
    Page,
    /// A device with no keyboard, whose user logs in on a phone. Nothing
    /// can go back to it, the poll works with the verifier alone. Allowed
    /// only for a challenge that a live short code leads to.
    Code,
}

impl HandBack {
    /// The way back of a start link, `port=` or `page=1`. `None` when the
    /// link has neither.
    pub(crate) fn from_query(port: Option<u16>, page: bool) -> Result<Option<Self>> {
        match (port, page) {
            (Some(_), true) => bail!("a login goes back by a port or to a page, not both"),
            (Some(0), false) => bail!("port 0 is no port"),
            (Some(port), false) => Ok(Some(Self::Loopback(port))),
            (None, true) => Ok(Some(Self::Page)),
            (None, false) => Ok(None),
        }
    }

    /// The text kept in `pending_logins.hand_back`.
    pub(crate) fn text(self) -> String {
        match self {
            Self::Loopback(port) => format!("port:{port}"),
            Self::Page => "page".to_owned(),
            Self::Code => "code".to_owned(),
        }
    }

    pub(crate) fn parse(text: &str) -> Result<Self> {
        match text {
            "page" => Ok(Self::Page),
            "code" => Ok(Self::Code),
            _ => match text.strip_prefix("port:") {
                Some(port) => Ok(Self::Loopback(port.parse()?)),
                None => bail!("no such way back of a login: {text}"),
            },
        }
    }

    /// Where the browser goes with the code.
    pub(crate) fn loopback_url(port: u16, code: &str) -> String {
        format!("http://127.0.0.1:{port}/?code={code}")
    }

    /// The `localStorage` key a page reads its code from. The client in
    /// `hilen/src/login/hand_back.rs` reads the same key.
    pub(crate) fn storage_key(challenge: &str) -> String {
        format!("hilen-login-{challenge}")
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::HandBack;

    #[test]
    fn the_text_of_each_way_back_reads_back() -> Result<()> {
        for hand_back in [HandBack::Loopback(51234), HandBack::Page, HandBack::Code] {
            assert_eq!(HandBack::parse(&hand_back.text())?, hand_back);
        }
        assert!(HandBack::parse("port:x").is_err());
        assert!(HandBack::parse("mail").is_err());
        Ok(())
    }

    #[test]
    fn a_start_link_names_1_way_back() -> Result<()> {
        assert_eq!(
            HandBack::from_query(Some(80), false)?,
            Some(HandBack::Loopback(80))
        );
        assert_eq!(HandBack::from_query(None, true)?, Some(HandBack::Page));
        assert_eq!(HandBack::from_query(None, false)?, None);
        assert!(HandBack::from_query(Some(80), true).is_err());
        assert!(HandBack::from_query(Some(0), false).is_err());
        Ok(())
    }
}
