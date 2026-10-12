//! Account selection by id, or brokerage name + last digits of the account number.

use crate::mask::last_n;
use crate::models::Account;

#[derive(Clone, Debug, Default)]
pub struct Selector {
    pub id: Option<String>,
    pub brokerage: Option<String>,
    pub last4: Option<String>,
}

impl Selector {
    pub fn is_empty(&self) -> bool {
        self.id.is_none() && self.brokerage.is_none() && self.last4.is_none()
    }

    pub fn matches(&self, account: &Account) -> bool {
        if let Some(id) = &self.id {
            if account.id.as_str() != id.as_str() {
                return false;
            }
        }
        if let Some(b) = &self.brokerage {
            let inst = account.institution_name.as_deref().unwrap_or("");
            if !inst.to_lowercase().contains(&b.to_lowercase()) {
                return false;
            }
        }
        if let Some(l) = &self.last4 {
            let want = last_n(l, l.chars().count());
            let number = account.number.as_deref().unwrap_or("");
            let name = account.name.as_deref().unwrap_or("");
            let hit = |s: &str| !want.is_empty() && last_n(s, want.len()) == want;
            if !(hit(number) || hit(name)) {
                return false;
            }
        }
        true
    }

    pub fn filter<'a>(&self, accounts: &'a [Account]) -> Vec<&'a Account> {
        accounts.iter().filter(|a| self.matches(a)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn accounts() -> Vec<Account> {
        serde_json::from_value(json!([
            {"id":"a1","institution_name":"Robinhood","name":"Robinhood Credit Card","number":"883821"},
            {"id":"a2","institution_name":"Schwab","name":"Investor Checking ...449","number":"Investor Checking ...449"},
            {"id":"a3","institution_name":"Schwab","name":"Brokerage ...3821","number":"Brokerage ...3821"}
        ])).unwrap()
    }

    #[test]
    fn by_brokerage_and_last4() {
        let a = accounts();
        let s = Selector {
            brokerage: Some("robin".into()),
            last4: Some("3821".into()),
            id: None,
        };
        let m = s.filter(&a);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].id.as_str(), "a1");
    }

    #[test]
    fn last4_across_brokerages_and_short_suffix() {
        let a = accounts();
        let s = Selector {
            last4: Some("3821".into()),
            ..Default::default()
        };
        assert_eq!(s.filter(&a).len(), 2);
        let s = Selector {
            brokerage: Some("schwab".into()),
            last4: Some("449".into()),
            id: None,
        };
        assert_eq!(s.filter(&a)[0].id.as_str(), "a2");
    }

    #[test]
    fn by_id_and_empty() {
        let a = accounts();
        let s = Selector {
            id: Some("a3".into()),
            ..Default::default()
        };
        assert_eq!(s.filter(&a).len(), 1);
        assert!(Selector::default().is_empty());
        assert_eq!(Selector::default().filter(&a).len(), 3);
    }
}
