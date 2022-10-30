//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! pinyin = "0.9"
//! chinese_segmenter = "1.0.1"
//! ```
//! 
//! 

use pinyin::{ToPinyin};
use term_macros::*;
use chinese_segmenter::{initialize, tokenize};

fn main() {
    initialize();
    readin!(wtr, |line: &[u8]| {
        let _ = std::str::from_utf8(line).map(|line| {
            tokenize(line).into_iter().for_each(|word| {
                for (i, pinyin) in word.to_pinyin().enumerate() {
                    if let Some(pinyin) = pinyin {
                        if i % 2 != 0 {
                            let _ = wtr.write_all(b"-");
                        }
                        let _ = wtr.write_all(pinyin.with_tone_num().as_bytes());
                        
                    }
                }
                let r = wtr.write_all(b" ");
                if r.is_err() {
                    panic!("Unable to write")
                }
            });
            let _ = wtr.write_all(b"\n");
        });
    });
}