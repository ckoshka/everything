//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! ```
use term_macros::*;
//use std::iter::FromIterator;
fn main() {

    tool! {
        args:
            - min_words: usize = 0;
            - min_chars: usize = 0;
            - max_words: usize = 1000000;
                ? max_words == 0
                => "max_words can't be zero dumbass"
            - max_chars: usize = max_words * 15;
        ;

        body: || {
            filter_in!(|line: &[u8]| {
                let word_count = line.split(|c| c == &b' ').count();
                !(line.len() > max_chars || line.len() < min_chars || word_count > max_words || word_count < min_words)
            });
        }

    };
}