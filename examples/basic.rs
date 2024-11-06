use std::io::{self, BufRead};

use ironlobe::{
    book::{btree_book::BTreeBook, Book},
    order::PlainOrder,
};

fn main() -> eyre::Result<()> {
    let stdin = io::stdin();

    let mut book: BTreeBook<PlainOrder> =
        BTreeBook::new(1, "Basic".to_string(), "BAS".to_string());
    let mut received_order = None;

    while let Some(Ok(line)) = stdin.lock().lines().next() {
        if line.trim() == "exit" {
            break;
        }

        match serde_json::from_str(&line) {
            Ok(order) => received_order = Some(book.add(order)?),
            Err(e) => println!("Malformed order JSON: {e:?}"),
        }

        println!("{:?}", received_order);
        println!("{}", book);
    }

    Ok(())
}
