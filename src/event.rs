use chrono::{DateTime, Utc};

use crate::{common::Quantity, order::Order};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchInfo<T: Order> {
    pub incumbent: T,
    pub others: Vec<(T, Quantity)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Match<T: Order> {
    Full(MatchInfo<T>),
    Partial(MatchInfo<T>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKind<T: Order> {
    Post(T),
    Match(Match<T>),
    Cancel(T),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event<T: Order> {
    pub timestamp: DateTime<Utc>,
    pub kind: EventKind<T>,
}

impl<T> Event<T>
where
    T: Order,
{
    pub fn new(kind: EventKind<T>) -> Self {
        Self {
            timestamp: Utc::now(),
            kind,
        }
    }
}
