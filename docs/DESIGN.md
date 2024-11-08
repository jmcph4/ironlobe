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

#### Performance ####

##### Asymptotic #####

When considering insertion into the book specifically, there are two cases:

 - Price level already exists, so $\mathcal{O}\left(1\right)$ as
    - Retrieval from the B-tree store ($\mathcal{O}\left(1\right)$)
    - Insertion into the deque ($\mathcal{O}\left(1\right)$)
 - Price level does not already exist, so $\mathcal{O}\left(\log{n}\right)$ as
    - Insertion into the B-tree store ($\mathcal{O}\left(\log{n}\right)$)
    - Insertion into the deque ($\mathcal{O}\left(1\right)$)

Note that submitting an order to the book (i.e., via `Book<T>::add`) is not purely insertion. This is obvious because if the order crosses the spread then matching will occur (this may result in an eventual insertion but will involve multiple retrievals and possibly deletions).

| Operation | Asymptotic Performance |
| --- | --- |
| `<BTreeBook<T> as Book<T>>::add` | Complicated, TODO |
| `<BTreeBook<T> as Book<T>>::cancel` | TODO |
| `<BTreeBook<T> as Book<T>>::top` | $\mathcal{O}\left(1\right)$ |
| `<BTreeBook<T> as Book<T>>::depth` | $\mathcal{O}\left(1\right)$ |
| `<BTreeBook<T> as Book<T>>::ltp` | $\mathcal{O}\left(1\right)$ |


##### Concrete #####

Consult the relevant [Criterion](https://docs.rs/criterion/latest/criterion) benchmarks in the repository. On a fairly unremarkable consumer-grade ThinkPad `BTreeBook` achieves an average time of **566ns** per order for 1,000 orders.

