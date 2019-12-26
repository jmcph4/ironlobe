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


#[cfg(test)]
mod tests { 
    use super::*;
    use std::collections::HashMap;
    use std::iter::FromIterator;
    use crate::account::*;

    #[test]
    fn test_new() -> Result<(), BookError> {
        let id: u128 = 1;
        let name: String = "Book".to_string();
        let ticker: String = "BOOK".to_string();

        let actual_book: Book = Book::new(id, name.clone(), ticker.clone());
        let expected_book: Book = Book{
            id: id,
            name: name.clone(),
            ticker: ticker.clone(),
            orders: HashMap::new(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: 0.00,
            has_traded: false
        };

        assert_eq!(actual_book, expected_book);
        Ok(())
    }

    #[test]
    fn test_submit_single_bid() -> Result<(), BookError> {
        /* build account */
        let account_id: AccountId = 1;
        let account_name: String = "Account".to_string();
        let account_balance: f64 = 12000.00;
        let account_holdings: HashMap<String, u128> = HashMap::new();
        let actual_account: Account = Account::new(account_id,
                                                   account_name,
                                                   account_balance,
                                                   account_holdings);

        /* build order */
        let order_id: OrderId = 1;
        let order_owner: Account = actual_account;
        let order_ticker: String = "BOOK".to_string();
        let order_type: OrderType = OrderType::Bid;
        let order_price: f64 = 12.00;
        let order_quantity: u128 = 33;
        let actual_order: Order = Order::new(order_id,
                                                 order_owner,
                                                 order_ticker,
                                                 order_type,
                                                 order_price,
                                                 order_quantity);

        /* build book */
        let book_id: BookId = 1;
        let book_name: String = "Book".to_string();
        let book_ticker: String = "BOOK".to_string();
        let mut actual_book: Book = Book::new(book_id,
                                              book_name.clone(),
                                              book_ticker.clone());

        /* we need to build this field of the expected book due to movement
         * of values */
        let mut expected_orders: HashMap<OrderId, Order> = HashMap::new();
        expected_orders.insert(order_id, actual_order.clone());
 
        /* submit order to book */
        actual_book.submit(actual_order)?;

        /* build expected fields */
        let mut cloned_expected_orders: HashMap<OrderId, Order> =
            expected_orders.clone();
        let mut expected_bids: BTreeMap<OrderedFloat<f64>,
        VecDeque<&mut Order>> =
            BTreeMap::new();
        expected_bids.insert(OrderedFloat::from(order_price),
            VecDeque::from_iter(
                vec![cloned_expected_orders.get_mut(&order_id).unwrap()]));

        let expected_asks: BTreeMap<OrderedFloat<f64>,
        VecDeque<&mut Order>> =
            BTreeMap::new();

        let expected_book: Book = Book {
            id: book_id,
            name: book_name.clone(),
            ticker: book_ticker.clone(),
            orders: expected_orders,
            bids: expected_bids,
            asks: expected_asks,
            ltp: 0.00,
            has_traded: false
        };

        assert_eq!(actual_book, expected_book);
        Ok(())
    }

    #[test]
    fn test_submit_single_ask() -> Result<(), BookError> {
        /* build account */
        let account_id: AccountId = 1;
        let account_name: String = "Account".to_string();
        let account_balance: f64 = 12000.00;
        let account_holdings: HashMap<String, u128> = HashMap::new();
        let actual_account: Account = Account::new(account_id,
                                                   account_name,
                                                   account_balance,
                                                   account_holdings);

        /* build order */
        let order_id: OrderId = 1;
        let order_owner: Account = actual_account;
        let order_ticker: String = "BOOK".to_string();
        let order_type: OrderType = OrderType::Ask;
        let order_price: f64 = 12.00;
        let order_quantity: u128 = 33;
        let actual_order: Order = Order::new(order_id,
                                                 order_owner,
                                                 order_ticker,
                                                 order_type,
                                                 order_price,
                                                 order_quantity);

        /* build book */
        let book_id: BookId = 1;
        let book_name: String = "Book".to_string();
        let book_ticker: String = "BOOK".to_string();
        let mut actual_book: Book = Book::new(book_id,
                                              book_name.clone(),
                                              book_ticker.clone());

        /* we need to build this field of the expected book due to movement
         * of values */
        let mut expected_orders: HashMap<OrderId, Order> = HashMap::new();
        expected_orders.insert(order_id, actual_order.clone());
 
        /* submit order to book */
        actual_book.submit(actual_order)?;

        /* build expected fields */
        let expected_bids: BTreeMap<OrderedFloat<f64>,
        VecDeque<&mut Order>> =
            BTreeMap::new();

        let mut cloned_expected_orders: HashMap<OrderId, Order> =
            expected_orders.clone();
        let mut expected_asks: BTreeMap<OrderedFloat<f64>,
        VecDeque<&mut Order>> =
            BTreeMap::new();
        expected_asks.insert(OrderedFloat::from(order_price),
            VecDeque::from_iter(
                vec![cloned_expected_orders.get_mut(&order_id).unwrap()]));

        let expected_book: Book = Book {
            id: book_id,
            name: book_name.clone(),
            ticker: book_ticker.clone(),
            orders: expected_orders,
            bids: expected_bids,
            asks: expected_asks,
            ltp: 0.00,
            has_traded: false
        };

        assert_eq!(actual_book, expected_book);
        Ok(())
    }
}

