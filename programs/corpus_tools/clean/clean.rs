//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! ```
use term_macros::*;
use std::collections::HashSet;

fn main() {
    tool! {
        args:
            - no_punctuation;
            - no_numbers;
            - lowercase;
            - ignore: String = "".to_string();
        ;

        body: || {
            let mut buffer = String::with_capacity(1000);
            let ignored_chars = ignore.chars().collect::<HashSet<_>>();
            readin!(wtr, |line: &[u8]| {
                let line = std::str::from_utf8(line);
                if line.is_err() {
                    return;
                }
                let line = line.unwrap();
                line
                    .trim()
                    .chars()
                    .map(|c| {
                        if ignored_chars.contains(&c) || c.is_whitespace() || c.is_alphabetic() || c.is_numeric() && !no_numbers || c.is_ascii_punctuation() && !no_punctuation {
                            c
                        } else {
                            ' '
                        }
                    })
                    .fold(&mut buffer, |accum, c| {
                        if accum
                            .chars()
                            .last()
                            .map(|c1| c1.is_whitespace())
                            .unwrap_or_else(|| false)
                            && c.is_whitespace()
                        {
                        } else {
                            match lowercase {
                                true => accum.extend(c.to_lowercase()),
                                false => accum.push(c)
                            };
                        }
                        accum
                    });
                if buffer.len() > 1 {
                    let _ = wtr.write_all(buffer.as_bytes());
                    let r = wtr.write_all(b"\n");
                    if r.is_err() {
                        panic!("Unable to write")
                    }
                }
                buffer.clear();
            });
        }

    };
}