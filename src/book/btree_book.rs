use std::collections::{BTreeMap, VecDeque};
use std::fmt::Display;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use eq_float::F64;
use eyre::ErrReport;
use serde::{Deserialize, Serialize};

use crate::event::{EventKind, Match, MatchInfo};
use crate::order::{OrderId, OrderKind};
use crate::{
    book::Book,
    common::{Price, Quantity},
    event::Event,
    order::Order,
};

use super::BookId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Levels {
    pub bids: Vec<(Price, Quantity)>,
    pub asks: Vec<(Price, Quantity)>,
}

/// Information about the market an order book represents
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    /// A unique identifier for the book
    id: BookId,
    /// The human-readable name of the market
    name: String,
    /// The abbreviated, human-readable identifier of the market
    ticker: String,
}

/// Limit order book where each side of the book is an ordered mapping (using
/// B-trees) keyed on price
#[derive(Clone, Debug)]
pub struct BTreeBook<T: Order> {
    /// Metadata for the market this book represents
    metadata: Metadata,
    /// Event log for this book (describes all mutations)
    events: Arc<RwLock<Vec<Event<T>>>>,
    /// Bid-side of the market
    bids: Arc<RwLock<BTreeMap<F64, VecDeque<T>>>>,
    /// Ask-side of the market
    asks: Arc<RwLock<BTreeMap<F64, VecDeque<T>>>>,
    /// Last Traded Price (LTP) of the book
    ltp: Arc<RwLock<Option<Price>>>,
    /// Total volume on each side of the book
    depth: Arc<RwLock<(Quantity, Quantity)>>,
}

/* custom impl to introspect locks */
impl<T> PartialEq for BTreeBook<T>
where
    T: Order,
{
    fn eq(&self, other: &Self) -> bool {
        self.metadata == other.metadata
            && *self.events.read().unwrap() == *other.events.read().unwrap()
            && *self.bids.read().unwrap() == *other.bids.read().unwrap()
            && *self.asks.read().unwrap() == *other.asks.read().unwrap()
            && *self.ltp.read().unwrap() == *other.ltp.read().unwrap()
            && *self.depth.read().unwrap() == *other.depth.read().unwrap()
    }
}

/* see above */
impl<T> Eq for BTreeBook<T> where T: Order {}

impl<T> Display for BTreeBook<T>
where
    T: Order,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bids_lock = self.bids.read().unwrap();
        let bids_iter = bids_lock.iter().rev().map(|(price, xs)| {
            (price.0, xs.iter().map(|x| x.quantity()).sum::<Quantity>())
        });
        let asks_lock = self.asks.read().unwrap();
        let asks_iter = asks_lock
            .iter()
            .map(|(price, xs)| {
                (price.0, xs.iter().map(|x| x.quantity()).sum::<Quantity>())
            })
            .rev();
        let bids: Vec<(Price, Quantity)> = bids_iter.collect();
        let asks: Vec<(Price, Quantity)> = asks_iter.collect();

        let col_width = 17;

        writeln!(f, "{:>17} | {:<17}", "BIDS", "ASKS")?;

        for ask in asks {
            writeln!(
                f,
                "{} | {:<8.2} {:<8.2}",
                " ".repeat(col_width),
                ask.0,
                ask.1
            )?;
        }

        for bid in bids {
            writeln!(f, "{:8.2} {:8.2} |", bid.0, bid.1)?;
        }

        Ok(())
    }
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
            ltp: Arc::new(RwLock::new(None)),
            depth: Arc::new(RwLock::new((
                Quantity::default(),
                Quantity::default(),
            ))),
        }
    }

    pub fn meta(metadata: Metadata) -> Self {
        BTreeBook {
            metadata,
            events: Arc::new(RwLock::new(vec![])),
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            ltp: Arc::new(RwLock::new(None)),
            depth: Arc::new(RwLock::new((
                Quantity::default(),
                Quantity::default(),
            ))),
        }
    }

    /// Given the price and side of the market, will an order cross the book?
    fn crosses(&self, price: Price, kind: OrderKind) -> bool {
        match kind {
            OrderKind::Bid => match self.top() {
                (_, Some(best_ask)) => price >= best_ask,
                _ => false,
            },
            OrderKind::Ask => match self.top() {
                (Some(best_bid), _) => price <= best_bid,
                _ => false,
            },
        }
    }

    /// Insert (post) an order to the book
    fn add_order(&mut self, order: T) {
        match order.kind() {
            OrderKind::Bid => {
                self.bids
                    .write()
                    .unwrap()
                    .entry(F64(order.price()))
                    .or_insert_with(|| VecDeque::from_iter(vec![]))
                    .push_back(order.clone());
                self.depth.write().unwrap().0 += order.quantity();
            }
            OrderKind::Ask => {
                self.asks
                    .write()
                    .unwrap()
                    .entry(F64(order.price()))
                    .or_insert_with(|| VecDeque::from_iter(vec![]))
                    .push_back(order.clone());
                self.depth.write().unwrap().1 += order.quantity();
            }
        }
        self.events.write().unwrap().push(Event {
            timestamp: Utc::now(),
            kind: EventKind::Post(order.clone()),
        });
    }

    pub fn levels(&self) -> Levels {
        Levels {
            bids: self
                .bids
                .read()
                .unwrap()
                .iter()
                .map(|(p, xs)| (p.0, xs.iter().map(|x| x.quantity()).sum()))
                .collect(),
            asks: self
                .asks
                .read()
                .unwrap()
                .iter()
                .map(|(p, xs)| (p.0, xs.iter().map(|x| x.quantity()).sum()))
                .collect(),
        }
    }

    fn r#match<'a>(
        &mut self,
        order: T,
        opposing_side: impl Iterator<Item = (&'a F64, &'a mut VecDeque<T>)>,
    ) where
        T: 'a,
    {
        let reduce_depth = |reduction| {
            let curr_depth = *self.depth.read().unwrap();
            match order.kind() {
                OrderKind::Bid => {
                    *self.depth.write().unwrap() =
                        (curr_depth.0, curr_depth.1 - reduction)
                }
                OrderKind::Ask => {
                    *self.depth.write().unwrap() =
                        (curr_depth.0 - reduction, curr_depth.1)
                }
            }
        };
        let mut ltp = order.price();
        let mut quantity_remaining = order.quantity();

        for (level, orders) in opposing_side {
            if quantity_remaining == 0 {
                break;
            }
            if *level <= F64(order.price()) {
                while let Some(incumbent) = orders.pop_front() {
                    if quantity_remaining > 0 {
                        let incumbent_quantity = incumbent.quantity();

                        if incumbent_quantity > quantity_remaining {
                            self.events.write().unwrap().push(Event::new(
                                EventKind::Match(Match::Full(MatchInfo {
                                    incumbent: order.clone(),
                                    others: vec![(
                                        incumbent.clone(),
                                        order.quantity(),
                                    )],
                                })),
                            ));
                        } else {
                            self.events.write().unwrap().push(Event::new(
                                EventKind::Match(Match::Full(MatchInfo {
                                    incumbent: incumbent.clone(),
                                    others: vec![(
                                        order.clone(),
                                        order.quantity(),
                                    )],
                                })),
                            ));
                        }

                        quantity_remaining -= incumbent_quantity;
                        ltp = incumbent.price();
                        reduce_depth(incumbent_quantity);
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        *self.ltp.write().unwrap() = Some(ltp);
    }

    fn prune(&self) {
        let bid_lock = self.bids.read().unwrap();
        dbg!("here");
        let bid_keys: Vec<F64> = bid_lock
            .iter()
            .filter(|(_, xs)| xs.is_empty())
            .map(|(p, _)| *p)
            .collect();
        drop(bid_lock);
        let ask_lock = self.asks.read().unwrap();
        let ask_keys: Vec<F64> = ask_lock
            .iter()
            .filter(|(_, xs)| xs.is_empty())
            .map(|(p, _)| *p)
            .collect();
        drop(ask_lock);

        bid_keys.iter().for_each(|p| {
            self.bids.write().unwrap().remove(p);
        });
        ask_keys.iter().for_each(|p| {
            self.asks.write().unwrap().remove(p);
        });
    }

    fn remove_order(&mut self, order_id: OrderId) {
        let bid_lock = self.bids.read().unwrap();
        let bid_pos: Vec<(F64, usize)> = bid_lock
            .iter()
            .map(|(price, level)| {
                (price, level.iter().position(|x| x.id() == order_id))
            })
            .filter(|(_, pos)| pos.is_some())
            .map(|(price, pos)| (*price, pos.unwrap()))
            .collect();
        drop(bid_lock);
        if let Some(bid) = bid_pos.first() {
            self.bids
                .write()
                .unwrap()
                .get_mut(&bid.0)
                .unwrap()
                .remove(bid.1);
        }

        let ask_lock = self.asks.read().unwrap();
        let ask_pos: Vec<(&F64, usize)> = ask_lock
            .iter()
            .map(|(price, level)| {
                (price, level.iter().position(|x| x.id() == order_id))
            })
            .filter(|(_, pos)| pos.is_some())
            .map(|(price, pos)| (price, pos.unwrap()))
            .collect();
        if let Some(ask) = ask_pos.first() {
            self.asks
                .write()
                .unwrap()
                .get_mut(ask.0)
                .unwrap()
                .remove(ask.1);
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

    fn order(&self, id: OrderId) -> Option<T> {
        self.bids
            .read()
            .unwrap()
            .values()
            .find(|xs| xs.iter().any(|x| x.id() == id))
            .and_then(|xs| xs.iter().find(|x| x.id() == id))
            .cloned()
    }

    fn add(&mut self, order: T) {
        if !self.crosses(order.price(), order.kind()) {
            self.add_order(order.clone());
        } else {
            match order.kind() {
                OrderKind::Bid => {
                    let asks = self.asks.clone();
                    let mut lock = asks.write().unwrap();
                    self.r#match(order, lock.iter_mut().rev());
                }
                OrderKind::Ask => {
                    let bids = self.bids.clone();
                    let mut lock = bids.write().unwrap();
                    self.r#match(order, lock.iter_mut().rev());
                }
            };
            self.prune();
        }
    }

    fn cancel(&mut self, order_id: crate::order::OrderId) -> Option<T> {
        let order = self.order(order_id)?;
        self.events.write().unwrap().push(Event {
            timestamp: Utc::now(),
            kind: EventKind::Cancel(order.clone()),
        });
        self.remove_order(order_id);
        Some(order)
    }

    fn ltp(&self) -> Option<Price> {
        *self.ltp.read().unwrap()
    }

    fn depth(&self) -> (Quantity, Quantity) {
        *self.depth.read().unwrap()
    }

    fn top(&self) -> (Option<Price>, Option<Price>) {
        (
            self.bids.read().unwrap().first_key_value().map(|x| x.0 .0),
            self.asks.read().unwrap().first_key_value().map(|x| x.0 .0),
        )
    }

    fn crossed(&self) -> bool {
        match self.top() {
            (Some(best_bid), Some(best_ask)) => best_ask > best_bid,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use eq_float::F64;

    use crate::order::PlainOrder;

    use super::*;

    fn mock_metadata() -> Metadata {
        let id: BookId = 1;
        let name: String = "Book".to_string();
        let ticker: String = "BOOK".to_string();

        Metadata { id, name, ticker }
    }

    #[test]
    fn test_new() {
        let actual_book: BTreeBook<PlainOrder> =
            BTreeBook::meta(mock_metadata());
        let expected_book = BTreeBook {
            metadata: mock_metadata(),
            events: Arc::new(RwLock::new(vec![])),
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            ltp: Arc::new(RwLock::new(None)),
            depth: Arc::new(RwLock::new((
                Quantity::default(),
                Quantity::default(),
            ))),
        };

        assert_eq!(actual_book, expected_book);
    }

    #[test]
    fn test_submit_single_bid() {
        let timestamp = Utc::now();

        let order = PlainOrder {
            id: 1,
            kind: OrderKind::Bid,
            price: 12.00,
            quantity: 10,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };
        let mut actual_book: BTreeBook<PlainOrder> =
            BTreeBook::meta(mock_metadata());
        actual_book.add(order.clone());
        let expected_book = BTreeBook {
            metadata: mock_metadata(),
            events: Arc::new(RwLock::new(vec![Event {
                timestamp,
                kind: EventKind::Post(order.clone()),
            }])),
            bids: Arc::new(RwLock::new(BTreeMap::from_iter(vec![(
                F64(12.00),
                VecDeque::from_iter(vec![order.clone()]),
            )]))),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            ltp: Arc::new(RwLock::new(None)),
            depth: Arc::new(RwLock::new((10, Quantity::default()))),
        };

        assert!(relaxed_structural_equal(actual_book, expected_book));
    }

    #[test]
    fn test_submit_single_ask() {
        let timestamp = Utc::now();

        let order = PlainOrder {
            id: 1,
            kind: OrderKind::Ask,
            price: 12.00,
            quantity: 10,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };
        let mut actual_book: BTreeBook<PlainOrder> =
            BTreeBook::meta(mock_metadata());
        actual_book.add(order.clone());
        let expected_book = BTreeBook {
            metadata: mock_metadata(),
            events: Arc::new(RwLock::new(vec![Event {
                timestamp,
                kind: EventKind::Post(order.clone()),
            }])),
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::from_iter(vec![(
                F64(12.00),
                VecDeque::from_iter(vec![order.clone()]),
            )]))),
            ltp: Arc::new(RwLock::new(None)),
            depth: Arc::new(RwLock::new((Quantity::default(), 10))),
        };

        assert!(relaxed_structural_equal(actual_book, expected_book));
    }

    #[test]
    fn test_submit_matching_bid_ask() {
        let timestamp = Utc::now();
        let price = 12.00;
        let quantity = 10;

        let bid = PlainOrder {
            id: 1,
            kind: OrderKind::Bid,
            price,
            quantity,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };
        let ask = PlainOrder {
            id: 2,
            kind: OrderKind::Ask,
            price,
            quantity,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };

        let mut actual_book: BTreeBook<PlainOrder> =
            BTreeBook::meta(mock_metadata());
        actual_book.add(bid.clone());
        assert!(actual_book.crosses(price, ask.kind()));
        actual_book.add(ask.clone());
        let expected_book = BTreeBook {
            metadata: mock_metadata(),
            events: Arc::new(RwLock::new(vec![
                Event {
                    timestamp,
                    kind: EventKind::Post(bid.clone()),
                },
                Event {
                    timestamp,
                    kind: EventKind::Match(Match::Full(MatchInfo {
                        incumbent: bid.clone(),
                        others: vec![(ask.clone(), quantity)],
                    })),
                },
            ])),
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            ltp: Arc::new(RwLock::new(Some(price))),
            depth: Arc::new(RwLock::new((
                Quantity::default(),
                Quantity::default(),
            ))),
        };

        dbg!(&actual_book);

        assert!(relaxed_structural_equal(actual_book, expected_book));
    }

    /// Given two [`BTreeBook`]s, determine if they are equal ignoring
    /// timestamps
    ///
    /// Specifically, ∀(l,r)∈(⟨Events_left⟩,⟨Events_right⟩),kind(l)==kind(r).
    fn relaxed_structural_equal<T>(
        left: BTreeBook<T>,
        right: BTreeBook<T>,
    ) -> bool
    where
        T: Order,
    {
        left.metadata == right.metadata
            && *left.bids.read().unwrap() == *right.bids.read().unwrap()
            && *left.asks.read().unwrap() == *right.asks.read().unwrap()
            && *left.ltp.read().unwrap() == *right.ltp.read().unwrap()
            && *left.depth.read().unwrap() == *right.depth.read().unwrap()
            && left
                .events
                .read()
                .unwrap()
                .iter()
                .map(|ev| ev.kind.clone())
                .collect::<Vec<EventKind<T>>>()
                == right
                    .events
                    .read()
                    .unwrap()
                    .iter()
                    .map(|ev| ev.kind.clone())
                    .collect::<Vec<EventKind<T>>>()
    }
}
