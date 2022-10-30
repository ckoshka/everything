use term_macros::*;
use unicode_segmentation::UnicodeSegmentation;
use itertools::Itertools;

fn main() {

    tool! {
        args: 
            - min_char_length: usize = 3;
            - col: usize = 0;
            - sep: String = "\t".to_string();
        ;

        body: || {
            filter_in!(|line: &[u8]| {
                std::str::from_utf8(line).ok().map(|ln| ln.split(&sep).nth(col)).flatten().map(|ln| ln.to_lowercase()).map(|ln| {
                    let words: Vec<_> = ln.unicode_words().filter(|w| w.len() >= min_char_length).collect();
                    words.iter().unique().count() - words.len() == 0
                }).unwrap_or_else(|| false)
            });
        }
    }
}
