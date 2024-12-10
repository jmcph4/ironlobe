use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Display;

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
    pub id: BookId,
    /// The human-readable name of the market
    pub name: String,
    /// The abbreviated, human-readable identifier of the market
    pub ticker: String,
}

/// Limit order book where each side of the book is an ordered mapping (using
/// B-trees) keyed on price
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BTreeBook<T: Order> {
    /// Metadata for the market this book represents
    metadata: Metadata,
    /// Event log for this book (describes all mutations)
    events: Vec<Event<T>>,
    /// Bid-side of the market
    bids: BTreeMap<F64, VecDeque<T>>,
    /// Ask-side of the market
    asks: BTreeMap<F64, VecDeque<T>>,
    /// Last Traded Price (LTP) of the book
    ltp: Option<F64>,
    /// Total volume on each side of the book
    depth: (Quantity, Quantity),
}

impl<T> Display for BTreeBook<T>
where
    T: Order,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bids_iter = self.bids.iter().rev().map(|(price, xs)| {
            (price.0, xs.iter().map(|x| x.quantity()).sum::<Quantity>())
        });
        let asks_iter = self
            .asks
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
        Self {
            metadata: Metadata { id, name, ticker },
            events: vec![],
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: None,
            depth: (0, 0),
        }
    }

    pub fn meta(metadata: Metadata) -> Self {
        Self {
            metadata,
            events: vec![],
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: None,
            depth: (0, 0),
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
                    .entry(F64(order.price()))
                    .or_insert_with(|| VecDeque::from_iter(vec![]))
                    .push_back(order.clone());
                self.depth.0 += order.quantity();
            }
            OrderKind::Ask => {
                self.asks
                    .entry(F64(order.price()))
                    .or_insert_with(|| VecDeque::from_iter(vec![]))
                    .push_back(order.clone());
                self.depth.1 += order.quantity();
            }
        }
        self.events.push(Event {
            timestamp: Utc::now(),
            kind: EventKind::Post(order.clone()),
        });
    }

    pub fn levels(&self) -> Levels {
        Levels {
            bids: self
                .bids
                .iter()
                .map(|(p, xs)| (p.0, xs.iter().map(|x| x.quantity()).sum()))
                .collect(),
            asks: self
                .asks
                .iter()
                .map(|(p, xs)| (p.0, xs.iter().map(|x| x.quantity()).sum()))
                .collect(),
        }
    }

    fn reduce_depth(
        depth: &mut (Quantity, Quantity),
        reduction: u64,
        kind: OrderKind,
    ) {
        match kind {
            OrderKind::Bid => *depth = (depth.0, depth.1 - reduction),
            OrderKind::Ask => *depth = (depth.0 - reduction, depth.1),
        }
    }

    fn r#match(&mut self, order: T) {
        let opposing_kind = order.kind().opposite();
        let opposing_side: Box<dyn Iterator<Item = (&F64, &mut VecDeque<T>)>> =
            match opposing_kind {
                OrderKind::Bid => Box::new(self.bids.iter_mut().rev()),
                OrderKind::Ask => Box::new(self.asks.iter_mut()),
            };

        let mut ltp = order.price();
        let mut quantity_remaining = order.quantity();

        for (level, orders) in opposing_side {
            if quantity_remaining == 0 {
                break;
            }
            if *level <= F64(order.price()) {
                while let Some(incumbent) = orders.iter_mut().next() {
                    if quantity_remaining > 0 {
                        let incumbent_quantity = incumbent.quantity();

                        match incumbent_quantity.cmp(&quantity_remaining) {
                            Ordering::Greater => {
                                self.events.push(Event::new(EventKind::Match(
                                    Match::Partial(MatchInfo {
                                        incumbent: incumbent.clone(),
                                        others: vec![(
                                            order.clone(),
                                            order.quantity(),
                                        )],
                                    }),
                                )));
                                *incumbent.quantity_mut() -= order.quantity();
                                quantity_remaining = 0;
                                Self::reduce_depth(
                                    &mut self.depth,
                                    order.quantity(),
                                    order.kind(),
                                );
                            }
                            Ordering::Equal => {
                                self.events.push(Event::new(EventKind::Match(
                                    Match::Full(MatchInfo {
                                        incumbent: incumbent.clone(),
                                        others: vec![(
                                            order.clone(),
                                            order.quantity(),
                                        )],
                                    }),
                                )));
                                quantity_remaining -= incumbent_quantity;
                                Self::reduce_depth(
                                    &mut self.depth,
                                    incumbent_quantity,
                                    order.kind(),
                                );
                                *incumbent.quantity_mut() = 0;
                            }
                            Ordering::Less => {
                                self.events.push(Event::new(EventKind::Match(
                                    Match::Full(MatchInfo {
                                        incumbent: incumbent.clone(),
                                        others: vec![(
                                            order.clone(),
                                            order.quantity(),
                                        )],
                                    }),
                                )));
                                quantity_remaining -= incumbent_quantity;
                                Self::reduce_depth(
                                    &mut self.depth,
                                    incumbent_quantity,
                                    order.kind(),
                                );
                                *incumbent.quantity_mut() = 0;
                            }
                        }

                        ltp = incumbent.price();
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        self.ltp = Some(F64(ltp));
    }

    fn remove_order_from_side(
        btree: &mut BTreeMap<F64, VecDeque<T>>,
        order_id: OrderId,
    ) {
        // Collect keys whose VecDeque becomes empty after removal.
        let mut empty_keys = Vec::new();

        for (&price, orders) in btree.iter_mut() {
            if let Some(pos) =
                orders.iter().position(|order| order.id() == order_id)
            {
                orders.remove(pos);
                if orders.is_empty() {
                    empty_keys.push(price);
                }
                break; // Exit early since IDs are unique.
            }
        }

        // Remove empty VecDeques from the map.
        for key in empty_keys {
            btree.remove(&key);
        }
    }

    fn remove_order(&mut self, order_id: OrderId) {
        Self::remove_order_from_side(&mut self.bids, order_id);
        Self::remove_order_from_side(&mut self.asks, order_id);
    }

    fn prune(&mut self) {
        let null_bids: Vec<OrderId> = self
            .bids
            .values_mut()
            .map(|level| {
                level
                    .iter()
                    .filter(|order| order.quantity() == 0)
                    .cloned()
                    .collect::<Vec<T>>()
            })
            .flatten()
            .map(|order| order.id())
            .collect();
        let null_asks: Vec<OrderId> = self
            .asks
            .values_mut()
            .map(|level| {
                level
                    .iter()
                    .filter(|order| order.quantity() == 0)
                    .cloned()
                    .collect::<Vec<T>>()
            })
            .flatten()
            .map(|order| order.id())
            .collect();

        null_bids.iter().for_each(|bid| self.remove_order(*bid));
        null_asks.iter().for_each(|ask| self.remove_order(*ask));
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
            .values()
            .find(|xs| xs.iter().any(|x| x.id() == id))
            .and_then(|xs| xs.iter().find(|x| x.id() == id))
            .cloned()
    }

    fn add(&mut self, order: T) {
        if !self.crosses(order.price(), order.kind()) {
            self.add_order(order.clone());
        } else {
            self.r#match(order);
            self.prune();
        }
    }

    fn cancel(&mut self, order_id: crate::order::OrderId) -> Option<T> {
        let order = self.order(order_id)?;
        self.events.push(Event {
            timestamp: Utc::now(),
            kind: EventKind::Cancel(order.clone()),
        });
        self.remove_order(order_id);
        Some(order)
    }

    fn ltp(&self) -> Option<Price> {
        self.ltp.map(|x| x.0)
    }

    fn depth(&self) -> (Quantity, Quantity) {
        self.depth
    }

    fn top(&self) -> (Option<Price>, Option<Price>) {
        (
            self.bids.first_key_value().map(|x| x.0 .0),
            self.asks.first_key_value().map(|x| x.0 .0),
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
            events: vec![Event {
                timestamp,
                kind: EventKind::Post(order.clone()),
            }],
            bids: BTreeMap::from_iter(vec![(
                F64(12.00),
                VecDeque::from_iter(vec![order.clone()]),
            )]),
            asks: BTreeMap::new(),
            ltp: None,
            depth: (10, Quantity::default()),
        };

        assert!(check_metadata(&actual_book, &expected_book));
        assert!(check_bids(&actual_book, &expected_book));
        assert!(check_asks(&actual_book, &expected_book));
        assert!(check_ltp(&actual_book, &expected_book));
        assert!(check_depth(&actual_book, &expected_book));
        assert!(check_event_logs(&actual_book, &expected_book));
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
            events: vec![Event {
                timestamp,
                kind: EventKind::Post(order.clone()),
            }],
            bids: BTreeMap::new(),
            asks: BTreeMap::from_iter(vec![(
                F64(12.00),
                VecDeque::from_iter(vec![order.clone()]),
            )]),
            ltp: None,
            depth: (Quantity::default(), 10),
        };

        assert!(check_metadata(&actual_book, &expected_book));
        assert!(check_bids(&actual_book, &expected_book));
        assert!(check_asks(&actual_book, &expected_book));
        assert!(check_ltp(&actual_book, &expected_book));
        assert!(check_depth(&actual_book, &expected_book));
        assert!(check_event_logs(&actual_book, &expected_book));
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
            events: vec![
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
            ],
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: (Some(F64(price))),
            depth: (Quantity::default(), Quantity::default()),
        };

        assert!(check_metadata(&actual_book, &expected_book));
        assert!(check_bids(&actual_book, &expected_book));
        assert!(check_asks(&actual_book, &expected_book));
        assert!(check_ltp(&actual_book, &expected_book));
        assert!(check_depth(&actual_book, &expected_book));
        assert!(check_event_logs(&actual_book, &expected_book));
    }

    #[test]
    fn test_submit_partially_matching_bid_ask() {
        let timestamp = Utc::now();
        let price = 12.00;
        let bid_quantity = 100;
        let ask_quantity = 12;

        let bid = PlainOrder {
            id: 1,
            kind: OrderKind::Bid,
            price,
            quantity: bid_quantity,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };
        let ask = PlainOrder {
            id: 2,
            kind: OrderKind::Ask,
            price,
            quantity: ask_quantity,
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
            events: vec![
                Event {
                    timestamp,
                    kind: EventKind::Post(bid.clone()),
                },
                Event {
                    timestamp,
                    kind: EventKind::Match(Match::Partial(MatchInfo {
                        incumbent: bid.clone(),
                        others: vec![(ask.clone(), ask_quantity)],
                    })),
                },
            ],
            bids: BTreeMap::from_iter(vec![(
                F64(price),
                VecDeque::from_iter(vec![{
                    let mut orig = bid.clone();
                    *orig.quantity_mut() = bid_quantity - ask_quantity;
                    orig
                }]),
            )]),
            asks: BTreeMap::new(),
            ltp: Some(F64(price)),
            depth: (bid_quantity - ask_quantity, Quantity::default()),
        };

        assert!(check_metadata(&actual_book, &expected_book));
        assert!(check_bids(&actual_book, &expected_book));
        assert!(check_asks(&actual_book, &expected_book));
        assert!(check_ltp(&actual_book, &expected_book));
        assert!(check_depth(&actual_book, &expected_book));
        assert!(check_event_logs(&actual_book, &expected_book));
    }

    #[test]
    fn test_submit_partially_matching_bid_ask_bid() {
        let timestamp = Utc::now();
        let price = 12.00;
        let bid_quantity = 100;
        let ask_quantity = 12;

        let bid1 = PlainOrder {
            id: 1,
            kind: OrderKind::Bid,
            price,
            quantity: bid_quantity,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };
        let ask = PlainOrder {
            id: 2,
            kind: OrderKind::Ask,
            price,
            quantity: ask_quantity,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };
        let bid2 = PlainOrder {
            id: 1,
            kind: OrderKind::Bid,
            price,
            quantity: bid_quantity,
            created: timestamp,
            modified: timestamp,
            cancelled: None,
        };

        let mut actual_book: BTreeBook<PlainOrder> =
            BTreeBook::meta(mock_metadata());
        actual_book.add(bid1.clone());
        assert!(actual_book.crosses(price, ask.kind()));
        actual_book.add(ask.clone());
        actual_book.add(bid2.clone());
        let expected_book = BTreeBook {
            metadata: mock_metadata(),
            events: vec![
                Event {
                    timestamp,
                    kind: EventKind::Post(bid1.clone()),
                },
                Event {
                    timestamp,
                    kind: EventKind::Match(Match::Partial(MatchInfo {
                        incumbent: bid1.clone(),
                        others: vec![(ask.clone(), ask_quantity)],
                    })),
                },
                Event {
                    timestamp,
                    kind: EventKind::Post(bid2.clone()),
                },
            ],
            bids: BTreeMap::from_iter(vec![(
                F64(price),
                VecDeque::from_iter(vec![
                    {
                        let mut orig = bid1.clone();
                        *orig.quantity_mut() = bid_quantity - ask_quantity;
                        orig
                    },
                    bid2.clone(),
                ]),
            )]),
            asks: BTreeMap::new(),
            ltp: Some(F64(price)),
            depth: (
                bid_quantity - ask_quantity + bid_quantity,
                Quantity::default(),
            ),
        };

        assert!(check_metadata(&actual_book, &expected_book));
        assert!(check_bids(&actual_book, &expected_book));
        assert!(check_asks(&actual_book, &expected_book));
        assert!(check_ltp(&actual_book, &expected_book));
        assert!(check_depth(&actual_book, &expected_book));
        assert!(check_event_logs(&actual_book, &expected_book));
    }

    #[test]
    fn test_submit_deep_cross() {
        let orders: Vec<PlainOrder> = vec![
            PlainOrder {
                id: 1,
                kind: OrderKind::Bid,
                price: 10.00,
                quantity: 120,
                created: Utc::now(),
                modified: Utc::now(),
                cancelled: None,
            },
            PlainOrder {
                id: 2,
                kind: OrderKind::Bid,
                price: 10.00,
                quantity: 300,
                created: Utc::now(),
                modified: Utc::now(),
                cancelled: None,
            },
            PlainOrder {
                id: 3,
                kind: OrderKind::Bid,
                price: 15.00,
                quantity: 300,
                created: Utc::now(),
                modified: Utc::now(),
                cancelled: None,
            },
            PlainOrder {
                id: 4,
                kind: OrderKind::Ask,
                price: 16.00,
                quantity: 100,
                created: Utc::now(),
                modified: Utc::now(),
                cancelled: None,
            },
            PlainOrder {
                id: 5,
                kind: OrderKind::Ask,
                price: 20.50,
                quantity: 230,
                created: Utc::now(),
                modified: Utc::now(),
                cancelled: None,
            },
            PlainOrder {
                id: 6,
                kind: OrderKind::Ask,
                price: 3.50,
                quantity: 1000,
                created: Utc::now(),
                modified: Utc::now(),
                cancelled: None,
            },
        ];

        let mut actual_book: BTreeBook<PlainOrder> =
            BTreeBook::meta(mock_metadata());

        orders.iter().for_each(|x| actual_book.add(x.clone()));

        let expected_book = BTreeBook {
            metadata: mock_metadata(),
            events: vec![
                Event {
                    timestamp: Utc::now(),
                    kind: EventKind::Post(orders[0].clone()),
                },
                Event {
                    timestamp: Utc::now(),
                    kind: EventKind::Post(orders[1].clone()),
                },
                Event {
                    timestamp: Utc::now(),
                    kind: EventKind::Post(orders[2].clone()),
                },
                Event {
                    timestamp: Utc::now(),
                    kind: EventKind::Post(orders[3].clone()),
                },
                Event {
                    timestamp: Utc::now(),
                    kind: EventKind::Post(orders[4].clone()),
                },
            ],
            bids: BTreeMap::new(),
            asks: BTreeMap::from_iter(vec![
                (
                    F64(orders[3].price()),
                    VecDeque::from_iter(vec![orders[3].clone()]),
                ),
                (
                    F64(orders[4].price()),
                    VecDeque::from_iter(vec![orders[4].clone()]),
                ),
                (
                    F64(orders[5].price()),
                    VecDeque::from_iter(vec![orders[5].clone()]),
                ),
            ]),
            ltp: Some(F64(10.00)),
            depth: (0, 510),
        };

        assert!(check_metadata(&actual_book, &expected_book));
        assert!(check_bids(&actual_book, &expected_book));
        assert!(check_asks(&actual_book, &expected_book));
        assert!(check_ltp(&actual_book, &expected_book));
        assert!(check_depth(&actual_book, &expected_book));
        assert!(check_event_logs(&actual_book, &expected_book));
    }

    /// ∀(l,r)∈(⟨left⟩,⟨right⟩),kind(l)==kind(r).
    fn check_event_logs<T>(left: &BTreeBook<T>, right: &BTreeBook<T>) -> bool
    where
        T: Order,
    {
        left.events
            .iter()
            .map(|ev| ev.kind.clone())
            .collect::<Vec<EventKind<T>>>()
            == right
                .events
                .iter()
                .map(|ev| ev.kind.clone())
                .collect::<Vec<EventKind<T>>>()
    }

    fn check_depth<T>(left: &BTreeBook<T>, right: &BTreeBook<T>) -> bool
    where
        T: Order,
    {
        left.depth == right.depth
    }

    fn check_ltp<T>(left: &BTreeBook<T>, right: &BTreeBook<T>) -> bool
    where
        T: Order,
    {
        left.ltp == right.ltp
    }

    fn check_bids<T>(left: &BTreeBook<T>, right: &BTreeBook<T>) -> bool
    where
        T: Order,
    {
        left.bids == right.bids
    }

    fn check_asks<T>(left: &BTreeBook<T>, right: &BTreeBook<T>) -> bool
    where
        T: Order,
    {
        left.asks == right.asks
    }

    fn check_metadata<T>(left: &BTreeBook<T>, right: &BTreeBook<T>) -> bool
    where
        T: Order,
    {
        left.metadata == right.metadata
    }
}
