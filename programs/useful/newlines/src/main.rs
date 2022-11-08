use term_macros::*;
use rayon::prelude::*;



fn main() {
    tool! {
        args:
            - input_file: String;
            - lines;
            - words;
            - chars;
        ;

        body: || {
            let map = mmap!(input_file);

            let lambda: &(dyn Fn(&&u8) -> bool + Send + Sync) = if lines {
                &|b: &&u8| **b == b'\n'
            } else if words {
                &|b: &&u8| **b == b' '
            } else if chars {
                &|_: &&u8| true
            } else {
                panic!("You need to specify either --lines / -l, --words / -w, or --chars / -c");
            };

            let total = map.par_iter()
                .filter(lambda)
                .count();
                
            println!("{}", total);
        }
    }
}
