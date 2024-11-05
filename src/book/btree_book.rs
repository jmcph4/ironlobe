use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, RwLock};

use eyre::ErrReport;

use crate::event::{EventKind, Match, MatchInfo};
use crate::order::OrderKind;
use crate::{
    book::Book,
    common::{Price, Quantity},
    event::Event,
    order::Order,
};

use super::BookId;

#[derive(Copy, Clone, Debug)]
pub enum BookError {
    OrderNotFound,
    SideEmpty,
    NoTrades,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    id: u64,
    name: String,
    ticker: String,
}

#[derive(Clone, Debug)]
pub struct BTreeBook<T: Order> {
    metadata: Metadata,
    events: Arc<RwLock<Vec<Event<T>>>>,
    bids: Arc<RwLock<BTreeMap<Price, VecDeque<T>>>>,
    asks: Arc<RwLock<BTreeMap<Price, VecDeque<T>>>>,
    ltp: Arc<RwLock<Price>>,
    depth: Arc<RwLock<(Quantity, Quantity)>>,
}

impl<T> BTreeBook<T>
where
    T: Order,
{
    pub fn new(id: BookId, name: String, ticker: String) -> Self {
        BTreeBook {
            metadata: Metadata { id, name, ticker },
            events: Arc::new(RwLock::new(vec![])),
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            ltp: Arc::new(RwLock::new(Price::default())),
            depth: Arc::new(RwLock::new((
                Quantity::default(),
                Quantity::default(),
            ))),
        }
    }
}

impl<T> Book<T> for BTreeBook<T>
where
    T: Order,
{
    type Error = ErrReport;

    fn id(&self) -> BookId {
        self.metadata.id
    }

    fn name(&self) -> String {
        self.metadata.name.clone()
    }

    fn ticker(&self) -> String {
        self.metadata.ticker.clone()
    }

    fn order(&self, id: crate::order::OrderId) -> Option<T> {
        self.bids
            .read()
            .unwrap()
            .values()
            .find(|xs| xs.iter().any(|x| x.id() == id))
            .and_then(|xs| xs.iter().find(|x| x.id() == id))
            .cloned()
    }

    fn add(&mut self, order: T) -> Result<T, Self::Error> {
        let mut matched = false;
        let mut quantity_remaining = order.quantity();

        match order.kind() {
            OrderKind::Bid => {
                for (level, orders) in self.asks.write().unwrap().iter_mut() {
                    if matched {
                        break;
                    }
                    if *level <= order.price() {
                        for incumbent in orders {
                            if quantity_remaining > 0 {
                                quantity_remaining -= incumbent.quantity();
                                if incumbent.quantity() >= quantity_remaining {
                                    self.events.write().unwrap().push(
                                        Event::new(EventKind::Match(
                                            Match::Full(MatchInfo {
                                                incumbent: order.clone(),
                                                others: vec![(
                                                    incumbent.clone(),
                                                    order.quantity(),
                                                )],
                                            }),
                                        )),
                                    )
                                } else {
                                    self.events.write().unwrap().push(
                                        Event::new(EventKind::Match(
                                            Match::Full(MatchInfo {
                                                incumbent: incumbent.clone(),
                                                others: vec![(
                                                    order.clone(),
                                                    order.quantity(),
                                                )],
                                            }),
                                        )),
                                    )
                                }
                            } else {
                                matched = true;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
            OrderKind::Ask => {
                for (level, orders) in self.asks.write().unwrap().iter_mut() {
                    if matched {
                        break;
                    }
                    if *level >= order.price() {
                        for incumbent in orders {
                            if quantity_remaining > 0 {
                                quantity_remaining -= incumbent.quantity();
                                if incumbent.quantity() >= quantity_remaining {
                                    self.events.write().unwrap().push(
                                        Event::new(EventKind::Match(
                                            Match::Full(MatchInfo {
                                                incumbent: order.clone(),
                                                others: vec![(
                                                    incumbent.clone(),
                                                    order.quantity(),
                                                )],
                                            }),
                                        )),
                                    )
                                } else {
                                    self.events.write().unwrap().push(
                                        Event::new(EventKind::Match(
                                            Match::Full(MatchInfo {
                                                incumbent: incumbent.clone(),
                                                others: vec![(
                                                    order.clone(),
                                                    order.quantity(),
                                                )],
                                            }),
                                        )),
                                    )
                                }
                            } else {
                                matched = true;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(order)
    }

    fn cancel(
        &mut self,
        order_id: crate::order::OrderId,
    ) -> Result<T, Self::Error> {
        todo!()
    }

    fn ltp(&self) -> Option<Price> {
        self.ltp.read().ok().map(|x| *x)
    }

    fn depth(&self) -> (Quantity, Quantity) {
        *self.depth.read().unwrap()
    }

    fn top(&self) -> (Price, Price) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_metadata() -> Metadata {
        let id: BookId = 1;
        let name: String = "Book".to_string();
        let ticker: String = "BOOK".to_string();

        Metadata { id, name, ticker }
    }

    #[test]
    fn test_new() -> Result<(), BookError> {
        let actual_book: BTreeBook<TestOrder> =
            BTreeBook::new(id, name.clone(), ticker.clone());
        let expected_book = BTreeBook {
            metadata: test_metadata(),
            events: Arc::new(RwLock::new(vec![])),
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            ltp: Arc::new(RwLock::new(Price::default())),
            depth: Arc::new(RwLock::new((
                Quantity::default(),
                Quantity::default(),
            ))),
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
        let actual_account: Account = Account::new(
            account_id,
            account_name,
            account_balance,
            account_holdings,
        );

        /* build order */
        let order_id: OrderId = 1;
        let order_owner: Account = actual_account;
        let order_ticker: String = "BOOK".to_string();
        let order_type: OrderKind = OrderKind::Bid;
        let order_price: f64 = 12.00;
        let order_quantity: u128 = 33;
        let actual_order: Order = Order::new(
            order_id,
            order_owner,
            order_ticker,
            order_type,
            order_price,
            order_quantity,
        );

        /* build book */
        let book_id: BookId = 1;
        let book_name: String = "Book".to_string();
        let book_ticker: String = "BOOK".to_string();
        let mut actual_book: Book =
            Book::new(book_id, book_name.clone(), book_ticker.clone());

        /* we need to build this field of the expected book due to movement
         * of values */
        let mut expected_orders: HashMap<OrderId, Order> = HashMap::new();
        expected_orders.insert(order_id, actual_order.clone());

        /* submit order to book */
        actual_book.submit(actual_order)?;

        /* build expected fields */
        let mut cloned_expected_orders: HashMap<OrderId, Order> =
            expected_orders.clone();
        let mut expected_bids: BTreeMap<
            OrderedFloat<f64>,
            VecDeque<&mut Order>,
        > = BTreeMap::new();
        expected_bids.insert(
            OrderedFloat::from(order_price),
            VecDeque::from_iter(vec![cloned_expected_orders
                .get_mut(&order_id)
                .unwrap()]),
        );

        let expected_asks: BTreeMap<OrderedFloat<f64>, VecDeque<&mut Order>> =
            BTreeMap::new();

        let expected_book: Book = Book {
            id: book_id,
            name: book_name.clone(),
            ticker: book_ticker.clone(),
            orders: expected_orders,
            bids: expected_bids,
            asks: expected_asks,
            ltp: 0.00,
            has_traded: false,
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
        let actual_account: Account = Account::new(
            account_id,
            account_name,
            account_balance,
            account_holdings,
        );

        /* build order */
        let order_id: OrderId = 1;
        let order_owner: Account = actual_account;
        let order_ticker: String = "BOOK".to_string();
        let order_type: OrderKind = OrderKind::Ask;
        let order_price: f64 = 12.00;
        let order_quantity: u128 = 33;
        let actual_order: Order = Order::new(
            order_id,
            order_owner,
            order_ticker,
            order_type,
            order_price,
            order_quantity,
        );

        /* build book */
        let book_id: BookId = 1;
        let book_name: String = "Book".to_string();
        let book_ticker: String = "BOOK".to_string();
        let mut actual_book: Book =
            Book::new(book_id, book_name.clone(), book_ticker.clone());

        /* we need to build this field of the expected book due to movement
         * of values */
        let mut expected_orders: HashMap<OrderId, Order> = HashMap::new();
        expected_orders.insert(order_id, actual_order.clone());

        /* submit order to book */
        actual_book.submit(actual_order)?;

        /* build expected fields */
        let expected_bids: BTreeMap<OrderedFloat<f64>, VecDeque<&mut Order>> =
            BTreeMap::new();

        let mut cloned_expected_orders: HashMap<OrderId, Order> =
            expected_orders.clone();
        let mut expected_asks: BTreeMap<
            OrderedFloat<f64>,
            VecDeque<&mut Order>,
        > = BTreeMap::new();
        expected_asks.insert(
            OrderedFloat::from(order_price),
            VecDeque::from_iter(vec![cloned_expected_orders
                .get_mut(&order_id)
                .unwrap()]),
        );

        let expected_book: Book = Book {
            id: book_id,
            name: book_name.clone(),
            ticker: book_ticker.clone(),
            orders: expected_orders,
            bids: expected_bids,
            asks: expected_asks,
            ltp: 0.00,
            has_traded: false,
        };

        assert_eq!(actual_book, expected_book);
        Ok(())
    }
}
