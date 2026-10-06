//! The linked accounts of a main account, the content of the accounts file in
//! the hidden Drive folder. Every device merges and writes it, so a merge
//! gives the same result in any order: per account the entry that changed
//! last wins, and a removed account stays as an entry with no token, or an
//! old copy of the file on another device would bring it back.

use std::fmt::{self, Debug, Formatter};

use serde::{Deserialize, Serialize};

use crate::google_access::{GoogleAccount, accounts::AccountInfo};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub subject:       String,
    pub email:         String,
    pub name:          Option<String>,
    pub picture:       Option<String>,
    /// Empty for a removed account.
    pub refresh_token: String,
    /// Unix seconds of the last change, a link, a new sign in or the removal.
    pub updated_at:    f64,
    #[serde(default)]
    pub removed:       bool,
}

impl Entry {
    pub(crate) fn of(account: &GoogleAccount, at: f64) -> Self {
        Self {
            subject:       account.subject.clone(),
            email:         account.email.clone(),
            name:          account.name.clone(),
            picture:       account.picture.clone(),
            refresh_token: account.refresh_token.clone(),
            updated_at:    at,
            removed:       false,
        }
    }

    pub(crate) fn info(&self) -> AccountInfo {
        AccountInfo {
            subject: self.subject.clone(),
            email:   self.email.clone(),
            name:    self.name.clone(),
            picture: self.picture.clone(),
            main:    false,
        }
    }
}

// Written by hand to keep the token out of every log line.
impl Debug for Entry {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Entry")
            .field("email", &self.email)
            .field("updated_at", &self.updated_at)
            .field("removed", &self.removed)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Vault {
    version:  u32,
    /// Sorted by subject, so two vaults with the same accounts are equal.
    accounts: Vec<Entry>,
}

impl Vault {
    pub(crate) const fn new() -> Self {
        Self {
            version:  1,
            accounts: Vec::new(),
        }
    }

    /// The accounts that are linked now.
    pub(crate) fn live(&self) -> impl Iterator<Item = &Entry> {
        self.accounts.iter().filter(|entry| !entry.removed)
    }

    /// Takes `entry` when it is newer than what is here for its account.
    pub(crate) fn put(&mut self, entry: Entry) {
        match self.accounts.binary_search_by(|have| have.subject.cmp(&entry.subject)) {
            Ok(at) if self.accounts[at].updated_at < entry.updated_at => self.accounts[at] = entry,
            Ok(_) => {}
            Err(at) => self.accounts.insert(at, entry),
        }
    }

    /// Marks an account removed and gives the token it had, `None` when it
    /// is not linked.
    pub(crate) fn remove(&mut self, subject: &str, at: f64) -> Option<String> {
        let entry = self
            .accounts
            .iter_mut()
            .find(|entry| entry.subject == subject && !entry.removed)?;
        entry.removed = true;
        // Never older than the entry it replaces, also with a wrong clock.
        entry.updated_at = at.max(entry.updated_at + 1.0);
        Some(std::mem::take(&mut entry.refresh_token))
    }

    pub(crate) fn merge(&mut self, other: Self) {
        for entry in other.accounts {
            self.put(entry);
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{from_str, to_string};

    use super::{Entry, Vault};

    fn entry(subject: &str, token: &str, at: f64) -> Entry {
        Entry {
            subject:       subject.to_owned(),
            email:         format!("{subject}@example.com"),
            name:          None,
            picture:       None,
            refresh_token: token.to_owned(),
            updated_at:    at,
            removed:       false,
        }
    }

    fn tokens(vault: &Vault) -> Vec<&str> {
        vault.live().map(|entry| entry.refresh_token.as_str()).collect()
    }

    #[test]
    fn a_merge_gives_the_same_vault_in_any_order() {
        let mut one = Vault::new();
        one.put(entry("b", "b-old", 10.0));
        one.put(entry("a", "a-1", 10.0));
        let mut two = Vault::new();
        two.put(entry("b", "b-new", 20.0));
        two.put(entry("c", "c-1", 5.0));

        let mut left = one.clone();
        left.merge(two.clone());
        let mut right = two;
        right.merge(one);

        assert_eq!(left, right);
        assert_eq!(tokens(&left), ["a-1", "b-new", "c-1"]);
    }

    #[test]
    fn a_removed_account_does_not_come_back_from_an_old_copy() {
        let mut old_copy = Vault::new();
        old_copy.put(entry("a", "a-1", 10.0));

        let mut here = old_copy.clone();
        assert_eq!(here.remove("a", 20.0).as_deref(), Some("a-1"));
        assert_eq!(here.remove("a", 30.0), None);

        here.merge(old_copy);
        assert_eq!(tokens(&here), Vec::<&str>::new());
    }

    #[test]
    fn a_new_sign_in_after_a_removal_links_the_account_again() {
        let mut vault = Vault::new();
        vault.put(entry("a", "a-1", 10.0));
        vault.remove("a", 20.0);
        vault.put(entry("a", "a-2", 30.0));
        assert_eq!(tokens(&vault), ["a-2"]);
    }

    #[test]
    fn a_removal_wins_over_the_entry_also_with_a_clock_that_is_behind() {
        let mut vault = Vault::new();
        vault.put(entry("a", "a-1", 100.0));
        vault.remove("a", 50.0);

        let mut other = Vault::new();
        other.put(entry("a", "a-1", 100.0));
        other.merge(vault);
        assert_eq!(tokens(&other), Vec::<&str>::new());
    }

    #[test]
    fn the_file_shape_and_no_token_in_a_debug_print() -> Result<()> {
        let mut vault = Vault::new();
        vault.put(entry("a", "secret-token", 10.0));
        let json = to_string(&vault)?;
        assert!(
            json.starts_with(r#"{"version":1,"accounts":[{"subject":"a""#),
            "{json}"
        );
        assert_eq!(from_str::<Vault>(&json)?, vault);
        assert!(!format!("{vault:?}").contains("secret-token"));
        Ok(())
    }
}
