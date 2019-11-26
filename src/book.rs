use std::collections::{BTreeMap, VecDeque};
use std::convert::TryInto;
extern crate ordered_float;

use ordered_float::OrderedFloat;

use crate::order;

pub enum BookError {
    SideEmpty,
    NoTrades,
}

type Side = BTreeMap<OrderedFloat<f64>, VecDeque<order::Order>>;

#[derive(Debug)]
pub struct Book {
    id: u128,
    name: String,
    ticker: String,
    bids: Side,
    asks: Side,
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

    pub fn submit(&mut self, order: order::Order) -> Result<(), BookError> {
        unimplemented!();
    }

    pub fn cancel(&mut self, order_id: u128) -> Result<(), BookError> {
        unimplemented!();
    }

    fn add_order(side: &mut Side, order: order::Order) -> Result<Side, BookError> {
        match order.get_order_type() {
            order::OrderType::Bid => {
                match side.get_mut(&OrderedFloat::from(order.get_price())) {
                    Some(level) => level.push_back(order),
                    None => {
                        side.insert(OrderedFloat::from(order.get_price()), VecDeque::new());
                        return Book::add_order(side, order);
                    }
                };
            },
            order::OrderType::Ask => {
                match side.get_mut(&OrderedFloat::from(order.get_price())) {
                    Some(level) => level.push_back(order),
                    None => {
                        side.insert(OrderedFloat::from(order.get_price()), VecDeque::new());
                        return Book::add_order(side, order);
                    }
                };
            }
        };

        Ok(side.clone())
    }

    fn execute_order(&self, order: &mut order::Order) {
        unimplemented!();
    }

    fn partially_execute_order(&self, order_id: u128, quantity: u128) {
        unimplemented!();
    }
}

impl PartialEq for Book {
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

