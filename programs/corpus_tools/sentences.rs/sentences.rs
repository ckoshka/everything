//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! ```
use std::collections::HashSet;
use term_macros::*;
//use std::iter::FromIterator;
//use std::sync::Arc;

fn main() {
    let delims = "..۔܁܂።᙮᠃᠉。..꓿꘎｡?¿;՞؟፧⁇⁈⁉≟⍰⸮꘏!¡՜߹႟᥄‼؟⁈܀፨";

    tool! {
        args:
            - add: String = String::new();
            - remove: String = String::new();
            - only_use: String = String::new();
            - reject_uncapitalized;
            - reject_unpunctuated;
            - min_length: usize = 1;
        ;

        body: || {

            let mut set = HashSet::new();

            match only_use.len() > 0 {
                false => {
                    delims.chars().for_each(|c| {set.insert(c);});
                    add.chars().for_each(|c| {set.insert(c);});
                    remove.chars().for_each(|c| {set.remove(&c);});
                }
                true => only_use.chars().for_each(|c| {set.insert(c);})
            };

            readin!(writer, |lns: &[u8]| {
                let lns = std::str::from_utf8(lns);
                if lns.is_err() {
                    return;
                }
                let lns = lns.unwrap();
                lns
                .split_inclusive(
                    |c| set.contains(&c)
                )
                .for_each(|line| {
                    if line.len() > min_length &&
                    (!reject_uncapitalized || line.chars().filter(|c| c.is_alphabetic()).next().map(|c| c.is_uppercase()).unwrap_or_else(|| false)) && (!reject_unpunctuated || line.chars().last().map(|c| !c.is_alphanumeric()).unwrap_or_else(|| false)) && !line.contains(" .") {
                        let _ = writer.write_all(line.replace("\n", "").as_bytes());
                        let r = writer.write_all(b"\n");
                        if r.is_err() {
                            panic!("Unable to write")
                        }
                    }
                });
            });
        }

    };
}
