use std::fmt::Debug;

use arbitrary::Arbitrary;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Price, Quantity};

pub mod plain;
pub use plain::*;

pub type OrderId = u128;

#[derive(Arbitrary, Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OrderKind {
    Bid,
    Ask,
}

pub trait Order: Clone + Debug + Eq + PartialEq {
    fn id(&self) -> OrderId;
    fn kind(&self) -> OrderKind;
    fn price(&self) -> Price;
    fn quantity(&self) -> Quantity;
    fn created_at(&self) -> DateTime<Utc>;
    fn modified_at(&self) -> DateTime<Utc>;
    fn cancelled_at(&self) -> Option<DateTime<Utc>>;
    fn cancelled(&self) -> bool;
}
