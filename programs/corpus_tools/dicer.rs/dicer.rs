//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! ```

use term_macros::*;

fn main() {
    tool! {
        args:
            - order: Vec<usize>;
            - sep: String = "\t".to_string();
            - newsep: String = "\t".to_string();
            - error_on_invalid_utf8;
        ;

        body: || {

            readin!(wtr, |line: &[u8]| {
                let line = std::str::from_utf8(line);
                if line.is_err() {
                    if error_on_invalid_utf8 {
                        panic!("Invalid utf-8");
                    } else {
                        let _ = wtr.write_all(b"\n");
                        return;
                    }
                }
                let line = line.unwrap();
                let line = &line[0..line.len() - 1];
                let parts: Vec<&str> = line.split(&sep).collect();
                let mut new_parts = Vec::with_capacity(order.len() * 2);
                for idx in order.iter() {
                    let part = parts.get(*idx);
                    if part.is_none() {
                        let _ = wtr.write_all(b"\n");
                        return;
                    }
                    let part = part.unwrap();
                    new_parts.push(*part);
                    new_parts.push(&newsep);
                };
                let _ = new_parts.pop();
                for part in new_parts.iter() {
                    let r = wtr.write_all(part.as_bytes());
                    if r.is_err() {
                        panic!("Unable to write")
                    }
                }
                if !new_parts.iter().last().unwrap().contains("\n") {
                    let _ = wtr.write_all(b"\n");
                }
                new_parts.clear();
            });
        }

    };
}