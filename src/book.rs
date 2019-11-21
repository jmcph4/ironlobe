use std::collections::{BTreeMap, VecDeque};
use std::convert::TryInto;
extern crate ordered_float;

use ordered_float::OrderedFloat;

use crate::order;

pub enum BookError {
    SideEmpty,
    NoTrades,
}

#[derive(Debug)]
pub struct Book {
    id: u128,
    name: String,
    ticker: String,
    bids: BTreeMap<OrderedFloat<f64>, VecDeque<order::Order>>,
    asks: BTreeMap<OrderedFloat<f64>, VecDeque<order::Order>>,
    ltp: f64,
    has_traded: bool
}

#[allow(dead_code, unused_variables)]
impl Book {
    pub fn new(id: u128, name: String, ticker: String) -> Book {
        Book {
            id: id,
            name: name.clone(),
            ticker: ticker.clone(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: 0.00,
            has_traded: false
        }
    }

    pub fn get_id(&self) -> u128 {
        self.id
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_ticker(&self) -> String {
        self.ticker.clone()
    }

    pub fn get_ltp(&self) -> Result<f64, BookError> {
        if self.has_traded {
            Ok(self.ltp)
        } else {
            Err(BookError::NoTrades)
        }
    }

    pub fn get_top(&self, side: order::OrderType) -> Result<f64, BookError> {
        if self.get_depth(side.clone()) == 0 { /* bounds check */
            return Err(BookError::SideEmpty);
        }

        match side {
            order::OrderType::Bid => 
                Ok(self.bids.keys().next().unwrap().into_inner()),
            order::OrderType::Ask =>
                Ok(self.asks.keys().next_back().unwrap().into_inner())
        }
    }

    pub fn get_depth(&self, side: order::OrderType) -> u128 {
        match side {
            order::OrderType::Bid => self.bids.len().try_into().unwrap(),
            order::OrderType::Ask => self.asks.len().try_into().unwrap()
        }
    }

    pub fn submit(&self, order: order::Order) -> Result<(), BookError> {
        unimplemented!();
    }

    pub fn cancel(&self, order_id: u128) -> Result<(), BookError> {
        unimplemented!();
    }

    fn add_order(&self, order: order::Order) {
        unimplemented!();
    }

    fn execute_order(&self, order_id: u128) {
        unimplemented!();
    }

    fn partially_execute_order(&self, order_id: u128, quantity: u128) {
        unimplemented!();
    }
}

