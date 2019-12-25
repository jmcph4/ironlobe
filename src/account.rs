use std::collections::HashMap;

#[derive(Debug)]
pub enum AccountError {
    AssetNotFound,
}

#[derive(Debug, Default, Clone, PartialEq)]
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

    pub fn set_id(&mut self, id: u128) {
        self.id = id;
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_balance(&self) -> f64 {
        self.balance
    }

    pub fn set_balance(&mut self, balance: f64) {
        self.balance = balance
    }

    pub fn add_balance(&mut self, balance: f64) {
        self.balance += balance
    }

    pub fn take_balance(&mut self, balance: f64) {
        self.balance -= balance;
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

    pub fn set_holding(&mut self, ticker: String, quantity: u128) -> 
        Result<(), AccountError> {
        if self.holds(ticker.clone()) {
            self.holdings.remove(&ticker);
            self.holdings.insert(ticker, quantity);
        } else {
            return Err(AccountError::AssetNotFound);
        }

        Ok(())
    }
}

