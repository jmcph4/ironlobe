# Design #

## Goals ##

Ironlobe has the following design goals:

 - **Speed**, Ironlobe seeks to minimise order matching latency
 - **Extensibility**, Ironlobe seeks to support a wide variety of different use cases
 - **Ergonomic**, Ironlobe seeks to be a joy to use

All order book implementations must be thread-safe and have good performance on the `Book::add` method.

## Approach ##

Where extension is encouraged, Ironlobe provides traits and generic functions on these traits. The most notable being,

 - `Order`, what an order looks like
 - `Book`, what an order book looks like

### `BTreeBook` ###

Currently, Ironlobe only has one implementation of an order book: the `BTreeBook`. The `BTreeBook` is an order book where each side of the market is a `BTreeMap` keyed on price. The values are double-ended queues (Rust's [`VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html)) containing `Order`s. The rationale for this is:

 - Amortised $\mathcal{O}\left(1\right)$ retrieval during the matching loop
 - Sorted-order traversal of price levels

Additionally, `BTreeBook` stores an event log which logs fills, posts, and cancellations with timestamps and in chronological order. `BTreeBook` also stores other information such as the last traded price (LTP) and the depth of the book.

