use std::io::{self, BufRead};

use clap::Parser;

use ironlobe::{
    book::{btree_book::BTreeBook, Book},
    order::PlainOrder,
};

#[derive(Clone, Debug, Parser)]
pub struct Opts {
    #[clap(long, action)]
    pub histogram: bool,
}

fn main() -> eyre::Result<()> {
    let opts = Opts::parse();
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
        if opts.histogram {
            println!("{}", serde_json::to_string(&book.levels())?);
        } else {
            print!("{}", book);
        }
        println!("{}", "-".repeat(80));
    }

    Ok(())
}
