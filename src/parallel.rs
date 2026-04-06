//! Read-mostly parallel helpers (optional **`rayon`** feature).
//!
//! The matching engine and each [`crate::book::PriceBook`] remain **single-writer**; this module
//! is for **aggregating** best bid/ask (or similar) across many books/locks in parallel.

use crate::book::PriceBook;
use crate::types::Price;

use rayon::prelude::*;

/// One book’s best bid and ask.
pub type BestBidAsk = (Option<Price>, Option<Price>);

/// Best bid/ask per book, computed in parallel (each `PB` must be safe to share across threads).
pub fn par_best_quotes<PB>(books: &[PB]) -> Vec<BestBidAsk>
where
    PB: PriceBook + Sync,
{
    books
        .par_iter()
        .map(|b| (b.best_bid(), b.best_ask()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::PriceBook;
    use crate::book::service::BTreeOrderBook;
    use crate::types::Side;

    #[test]
    fn par_best_quotes_matches_sequential() {
        let mut a = BTreeOrderBook::new();
        let mut b = BTreeOrderBook::new();
        a.push(&10, 1, Side::Buy, 0);
        b.push(&20, 2, Side::Sell, 0);
        let books = [a, b];
        let seq: Vec<_> =
            books.iter().map(|x| (x.best_bid(), x.best_ask())).collect();
        let par = par_best_quotes(&books);
        assert_eq!(par, seq);
    }
}
