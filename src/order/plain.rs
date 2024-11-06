use chrono::{DateTime, Utc};
use eq_float::F64;
use serde::{Deserialize, Serialize};

use crate::common::{Price, Quantity};

use super::{Order, OrderId, OrderKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlainOrder {
    pub id: OrderId,
    pub kind: OrderKind,
    pub price: Price,
    pub quantity: Quantity,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub cancelled: Option<DateTime<Utc>>,
}

impl PartialEq for PlainOrder {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.kind == other.kind
            && F64(self.price) == F64(other.price)
            && self.quantity == other.quantity
            && self.created == other.created
            && self.modified == other.modified
            && self.cancelled == other.cancelled
    }
}

impl Eq for PlainOrder {}

impl Order for PlainOrder {
    fn id(&self) -> super::OrderId {
        self.id
    }

    fn kind(&self) -> super::OrderKind {
        self.kind
    }

    fn price(&self) -> crate::common::Price {
        self.price
    }

    fn quantity(&self) -> crate::common::Quantity {
        self.quantity
    }

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created
    }

    fn modified_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.modified
    }

    fn cancelled_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.cancelled
    }

    fn cancelled(&self) -> bool {
        self.cancelled.is_some()
    }
}
