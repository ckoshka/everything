use term_macros::*;
mod sort;
mod structs;
mod types;
use rayon::prelude::*;
use sort::*;
use structs::*;
use types::*;
use std::io::Write;
use std::io::Read;
macro_rules! push {
    ($boolean:ident ? $struc:expr => $vec:ident) => {
        if ($boolean) {
            $vec.push(Box::from($struc));
        }
    };
}

fn main() {
    tool! {
        args:
            - cutoff: usize = 5000;
            - penalise_capitals: bool = false;
            - out_of_freq: bool = false;
            - well_formed: bool = false;
            - redundancy: bool = false;
            - translated_partial: bool = false;
            - nonalphabetic: bool = false;
            - unicode_range: bool = false;
            //- ideal_length: f64 = 4.0;
            - length_difference: bool = false;
            - k_top: f64 = 0.75;
        ;

        body: || {
            let mut data = String::new();
            std::io::stdin().read_to_string(&mut data).unwrap();

            let lines: Vec<_> = data.par_split(|u| u == '\n')
                .collect();
            let mut txs: Vec<_> = lines
                .par_iter()
                .enumerate()
                .filter(|(_, line)| line.contains('\t') && line.len() > 5)
                .map(|(i, line)| {
                    let mut parts = line.split("\t");
                    Translation::new(parts.next().unwrap(), parts.next().unwrap(), i.try_into().unwrap())
                })
                .collect();

            let mut scorers: Vec<Box<dyn Sorter + Send + Sync>> = vec![];

            push!(redundancy ? Redundant => scorers);
            push!(translated_partial ? Untranslated => scorers);
            push!(nonalphabetic ? NonAlphabetic => scorers);
            push!(penalise_capitals ? Capitals => scorers);
            push!(well_formed ? WellFormed => scorers);
            push!(out_of_freq ? OutOfFrequency::from_txs(&txs, cutoff) => scorers);
            push!(unicode_range ? CharRange::from_txs(&txs) => scorers);

            //let use_ideal_length = true;
            //push!(use_ideal_length ? IdealLength(ideal_length) => scorers);
            push!(length_difference ? LengthDifference => scorers);

            let sorted_txs = sort(scorers, &mut txs);

            let stdout = std::io::stdout();
            let mut lock = stdout.lock();

            sorted_txs.iter().take((sorted_txs.len() as f64 * k_top).floor() as usize).for_each(|tx| {
                lock.write_all(&format!("{}\t{}\n", tx.sides.0.content, tx.sides.1.content).as_bytes()).unwrap();
            });
            lock.flush().unwrap();
        }
    }
}
