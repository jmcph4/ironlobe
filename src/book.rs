use std::collections::{HashMap, BTreeMap, VecDeque};
extern crate ordered_float;

use ordered_float::OrderedFloat;
use crate::order::*;

#[derive(Debug)]
#[allow(dead_code)]
pub enum BookError {
    OrderNotFound,
    SideEmpty,
    NoTrades,
}

pub type BookId = u128;
pub type PriceKey = OrderedFloat<f64>;

#[derive(Debug)]
pub struct Book<'a> {
    id: BookId,
    name: String,
    ticker: String,
    orders: HashMap<OrderId, Order>,
    bids: BTreeMap<PriceKey, VecDeque<&'a mut Order>>,
    asks: BTreeMap<PriceKey, VecDeque<&'a mut Order>>,
    ltp: f64,
    has_traded: bool
}

#[allow(dead_code, unused_variables)]
impl Book<'_> {
    pub fn new(id: u128, name: String, ticker: String) -> Book<'static> {
        Book {
            id: id,
            name: name,
            ticker: ticker,
            orders: HashMap::new(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: 0.00,
            has_traded: false
        }
    }

    pub fn get_id(&self) -> BookId {
        self.id
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_ticker(&self) -> String {
        self.ticker.clone()
    }

    pub fn get_order(&self, id: OrderId) -> Result<&Order, BookError> {
        match self.orders.get(&id) {
            Some(order) => Ok(order),
            None => Err(BookError::OrderNotFound)
        }
    }

    pub fn get_order_mut(&mut self, id: OrderId) ->
        Result<&mut Order, BookError> {
        match self.orders.get_mut(&id) {
            Some(order) => Ok(order),
            None => Err(BookError::OrderNotFound)
        }
    }

    pub fn get_ltp(&self) -> Result<f64, BookError> {
        if self.has_traded {
            Ok(self.ltp)
        } else {
            Err(BookError::NoTrades)
        }
    }

    #[allow(unused_mut)]
    pub fn submit(&mut self, mut order: Order) -> Result<(), BookError> {
        unimplemented!();
    }

    pub fn cancel(&mut self, id: OrderId) -> Result<(), BookError> {
        unimplemented!();
    }
}


impl PartialEq for Book<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id &&
            self.name == other.name &&
            self.ticker == other.ticker &&
            self.ltp == other.ltp &&
            self.has_traded == other.has_traded &&
            self.bids.iter().len() == other.bids.iter().len() &&
            self.asks.iter().len() == other.asks.iter().len() &&
            Vec::new().extend(self.bids.iter().map(|x| x)) == 
                Vec::new().extend(other.bids.iter().map(|x| x)) &&
            Vec::new().extend(self.asks.iter().map(|x| x)) == 
                Vec::new().extend(other.asks.iter().map(|x| x))
    }
}

