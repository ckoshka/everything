use std::io::{Error, ErrorKind};
use lz4_flex::compress_prepend_size;
use term_macros::*;
use memmap::MmapOptions;
use rayon::prelude::*;

fn compression_ratio<'a>(f1: &'a [u8]) -> impl Fn(&[u8]) -> Result<f64, Error> + 'a {
    let compr_f1_len = compress_prepend_size(f1).len();
    move |f2: &[u8]| {
        if f1.len() + f2.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidData, "Both slices are empty"));
        };

        let compr_f2_len = compress_prepend_size(f2).len();
        let compressed_together =
            compress_prepend_size(&f1.iter().chain(f2.iter()).copied().collect::<Vec<u8>>());

        Ok((compressed_together.len()) as f64 / (compr_f1_len + compr_f2_len) as f64)
    }
}

/// Splits the provided sequence into chunks of chunk_size.
/// Gets the compressed length of each chunk.
/// Gets the compressed length of the entire sequence.
/// Returns a Vec containing each slice along with its associated compression ratio.
fn measure_redundancy(bytes: &[u8], chunk_size: usize, max_consider: usize) -> Vec<(&[u8], f64)> {

    let indices = (0..(bytes.len() / chunk_size))
        .map(|i| i * chunk_size)
        .take(max_consider)
        .collect::<Vec<_>>();

    indices.into_par_iter()
        .map(move |idx| {
            let closest_newline_start = bytes
                .iter()
                .enumerate()
                .skip(idx)
                .find(|(_, b)| **b == b'\n')
                .map(|(i, _)| i)
                .unwrap_or_else(|| idx);
            let min = std::cmp::min(bytes.len(), idx + chunk_size);
            let closest_newline_end = bytes
                .iter()
                .enumerate()
                .skip(min)
                .find(|(_, b)| **b == b'\n')
                .map(|(i, _)| i)
                .unwrap_or_else(|| min);
            let without_chunk = [&bytes[..closest_newline_start], &bytes[closest_newline_end..]].concat();
            let fnc = compression_ratio(&without_chunk);
            let chunk = &bytes[closest_newline_start..closest_newline_end];
            let compressed_len = fnc(chunk)?;
            Ok((chunk, compressed_len))
        })
        .filter(|c: &Result<(&[_], _), Error>| c.is_ok())
        .map(|c| c.unwrap())
        .collect()
}

fn main() {
    tool! {
        args:
            - filename: String;
            - chunk_size: usize = 512;
            - max_consider: usize = 1000;
        ;

        body: || {
            let mmap = unsafe {
                std::fs::File::open(&filename).ok().and_then(|m| MmapOptions::new().map(&m).ok()).unwrap()
            };
            let mut rds = measure_redundancy(&mmap[..], chunk_size, max_consider);
            rds.sort_by_key(|t| (t.1 * 10000000000.0) as usize);
            rds.iter().rev().for_each(|tup| {
                println!("{}{}\n", tup.1, std::str::from_utf8(tup.0).unwrap());
            });
            // rev = repetitive, normal = diverse
        }
    }
    println!("Hello, world!");
}
