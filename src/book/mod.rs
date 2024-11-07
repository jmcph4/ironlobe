pub mod btree_book;

use std::fmt::Debug;

use crate::{
    common::{Price, Quantity},
    order::{Order, OrderId},
};

pub type BookId = u64;

pub trait Book<T: Order>: Clone + Debug {
    type Error;

    fn id(&self) -> BookId;
    fn name(&self) -> String;
    fn ticker(&self) -> String;
    fn order(&self, id: OrderId) -> Option<T>;
    fn add(&mut self, order: T);
    fn cancel(&mut self, order_id: OrderId) -> Option<T>;
    fn ltp(&self) -> Option<Price>;
    fn depth(&self) -> (Quantity, Quantity);
    fn top(&self) -> (Option<Price>, Option<Price>);
    fn crossed(&self) -> bool;
}
