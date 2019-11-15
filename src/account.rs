use std::collections::HashMap;

pub enum AccountError {
    AssetNotFound,
}

#[derive(Debug, Default)]
pub struct Account {
    id: u128,
    name: String,
    balance: f64,
    holdings: HashMap<String, u128>
}

#[allow(dead_code)]
impl Account {
    pub fn new(id: u128, name: String, balance: f64,
               holdings: HashMap<String, u128>) -> Account {
        Account {id, name, balance, holdings}
    }

    pub fn get_id(&self) -> u128 {
        self.id
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_balance(&self) -> f64 {
        self.balance
    }

    pub fn holds(&self, ticker: String) -> bool {
        self.holdings.contains_key(&ticker)
    }

    pub fn get_holding(&self, ticker: String) -> Result<u128, AccountError> {
        if self.holds(ticker.clone()) {
            Ok(self.holdings[&ticker])
        } else {
            Err(AccountError::AssetNotFound)
        }
    }
}

