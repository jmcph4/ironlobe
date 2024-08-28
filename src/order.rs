extern crate chrono;

use chrono::{DateTime, Utc};

use crate::account;

pub enum OrderError {
    OrderStillActive,
}

pub type OrderId = u128;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OrderType {
    Bid,
    Ask,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    id: u128,
    owner: account::Account,
    ticker: String,
    order_type: OrderType,
    price: f64,
    quantity: u128,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    cancelled: DateTime<Utc>,
    active: bool,
}

#[allow(dead_code)]
impl Order {
    pub fn new(
        id: u128,
        owner: account::Account,
        ticker: String,
        order_type: OrderType,
        price: f64,
        quantity: u128,
    ) -> Order {
        Order {
            id: id,
            owner: owner,
            ticker: ticker.clone(),
            order_type: order_type,
            price: price,
            quantity: quantity,
            created: Utc::now(),
            modified: Utc::now(),
            cancelled: Utc::now(),
            active: true,
        }
    }

    pub fn get_id(&self) -> u128 {
        self.id
    }

    pub fn get_owner(&self) -> account::Account {
        self.owner.clone()
    }

    pub fn get_owner_mut(&mut self) -> &mut account::Account {
        &mut self.owner
    }

    pub fn get_ticker(&self) -> String {
        self.ticker.clone()
    }

    pub fn get_order_type(&self) -> OrderType {
        self.order_type.clone()
    }

    pub fn get_price(&self) -> f64 {
        self.price
    }

    pub fn get_quantity(&self) -> u128 {
        self.quantity
    }

    pub fn get_created(&self) -> DateTime<Utc> {
        self.created
    }

    pub fn get_modified(&self) -> DateTime<Utc> {
        self.modified
    }

    pub fn get_cancelled(&self) -> Result<DateTime<Utc>, OrderError> {
        if self.active {
            Ok(self.cancelled)
        } else {
            Err(OrderError::OrderStillActive)
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }
}
