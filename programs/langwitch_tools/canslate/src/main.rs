use term_macros::*;
use unicode_segmentation::UnicodeSegmentation;
use std::io::prelude::*;
use std::hash::Hasher;
use std::hash::Hash;
use std::io::stdout;
use std::io::BufWriter;
use std::collections::HashSet;
type Filename = String;

// even simpler, just assume all the sentences provided are ones we know already.
// so we just need:
// text -> extract words | frqcheck <- words | canslate -c 0 | returns the output 

fn main() {
    tool! {
        args:
            - col_to_search: usize;
            - sep: String = "\t".to_string();
        ;

        body: || {
            let mut stdin = String::new();
            std::io::stdin().read_to_string(&mut stdin).unwrap();

            let words: HashSet<String> = stdin
                .split("\n")
                .map(|line| {
                    line.split(&sep).nth(col_to_search).unwrap().unicode_words().map(|w| w.to_lowercase())
                })
                .flatten()
                .collect();

            words.iter().for_each(|w| {
                println!("{}", w);
            });

        }
    }
}
