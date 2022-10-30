//!
//! ```cargo
//! [dependencies]
//! lz4_flex = { version = "0.9.0", default-features = false }
//! rayon = "1.5.3"
//! memmap = "0.7.0"
//! term_macros = { path = "../../shared/term_macros"  }
//! ```

use lz4_flex::compress_prepend_size;
use memmap::MmapOptions;
use rayon::prelude::*;
use std::io::{Error, ErrorKind};
use std::{collections::HashMap, path::PathBuf};
use term_macros::*;

fn main() {
    tool! {
        args:
            - interactive;
            - desired_lang: Option<String> = None;

            - reference_files: String = "/Users/ckoshka/programming/bash_experiments/showcase/curr/tools/languages_udhr".to_string();
            - also_include: Option<String> = None;
            - top_n: usize = 5;
            - sparsity: usize = if interactive {
                    1
                } else {
                    30
                };
            - min_confidence: f64 = 2.0;
            - confidence_ratio: Option<f64> = None;
        ;

        body: || {
            let maybe_desired_lang = desired_lang.unwrap_or_else(|| "".to_string());
            let lengths = get_lengths(&maybe_desired_lang, &reference_files, &sparsity, also_include);
            let average_length = lengths
                .values()
                .map(|(_, byte_length)| byte_length)
                .sum::<usize>() as f64
                / lengths.len() as f64;

            if interactive {
                readin!(_wtr, |sentence: &[u8]| {
                    let confidences = get_likelihood_of_lang(&lengths, sentence, average_length);
                    confidences
                        .iter()
                        .rev()
                        .take(top_n)
                        .for_each(|(lang, conf)| {
                            println!("{} = {}", lang.split("/").last().unwrap(), conf);
                        });
                });
            } else {
                if maybe_desired_lang == "" {
                    panic!("Desired lang wasn't specified!")
                }
                filter_in!(|sentence: &[u8]| {
                    let confidences = get_likelihood_of_lang(&lengths, sentence, average_length);
                    let mut iterator = confidences
                        .iter()
                        .rev();
                    if confidence_ratio.is_some() {
                        let l1 = iterator.next().unwrap();
                        let l2 = iterator.next().unwrap();
                        return confidence_ratio.map(|x| l1.0 == maybe_desired_lang && (l1.1 / l2.1) > x && l1.1 > min_confidence).unwrap();
                    } else {
                        iterator
                            .take(top_n)
                            .filter(|(_, c)| c > &min_confidence)
                            .find(|(l, _)| l == &maybe_desired_lang)
                            .is_some()
                    }
                });
            }
        }

    };
}

fn compression_ratio(
    f1: &[u8],
    f2: &[u8],
    compr_f1_len: usize,
    compr_f2_len: usize,
) -> Result<f64, Error> {
    if f1.len() + f2.len() == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "File is empty"));
    }
    let compressed_together =
        compress_prepend_size(&f1.iter().chain(f2.iter()).copied().collect::<Vec<u8>>());

    let actual = (compressed_together.len()) as f64 / (compr_f1_len + compr_f2_len) as f64;
    Ok(actual)
}

/// Returns a list of all files found in the top level of a directory (and not within any subdirectories). Ignores folders.
fn get_files(path: &str) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let files = std::fs::read_dir(path)?
        .into_iter()
        .map(|entry| {
            entry.and_then(|e| {
                if e.path().is_file() {
                    Ok(e.path().to_path_buf())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Not a file",
                    ))
                }
            })
        })
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    Ok(files)
}

fn get_lengths(
    desired_lang: &str,
    language_dir: &str,
    sparsity: &usize,
    also_include: Option<String>,
) -> HashMap<String, (memmap::Mmap, usize)> {
    let also_include = also_include
        .map(|filenames| {
            filenames
                .split(",")
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![]);
    let predicate = |p: &PathBuf| {
        let as_str = p.to_string_lossy().to_string();
        also_include
            .iter()
            .find(|filename| as_str == **filename)
            .is_some()
    };
    get_files(language_dir)
        .unwrap()
        .into_par_iter()
        .enumerate()
        .filter(|(i, c)| {
            (*i as i32).rem_euclid(*sparsity as i32) == 0
                || c.to_string_lossy().to_string() == desired_lang
                || predicate(c)
        })
        .map(|(_i, c)| c)
        .map(|fname| (fname.clone(), std::fs::File::open(fname).unwrap()))
        .map(|(fname, file)| {
            // Safety: no
            let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
            let byte_length = compress_prepend_size(&mmap[..]).len();
            (fname.to_string_lossy().to_string(), (mmap, byte_length))
        })
        .collect()
}

pub fn get_likelihood_of_lang<'a>(
    lengths: &'a HashMap<String, (memmap::Mmap, usize)>,
    input_bytes: &[u8],
    average_length: f64,
) -> Vec<(&'a str, f64)> {
    let compressed_length = compress_prepend_size(input_bytes).len();
    let compression_ratios = lengths
        .par_iter()
        .map(|(lang, (f1, compr_f1_len))| {
            let f2 = input_bytes;
            //let ratio = (*compr_f1_len as f64 - averaged_compressed_length) / (averaged_compressed_length);
            let result = compression_ratio(f1, f2, *compr_f1_len, compressed_length).unwrap();
            (result, lang.as_str())
        })
        .collect::<Vec<(f64, &str)>>();
    let (f1, _lang) = compression_ratios
        .iter()
        .max_by(|&(f1, _), &(f2, _)| f1.partial_cmp(&f2).unwrap())
        .unwrap();

    //let (f1, _lang) = compression_ratios.get(0).expect("No languages found");
    let f1 = 1.0 - *f1;
    let mut adjusted_ratios: Vec<_> = compression_ratios
        .iter()
        .map(|(confidence, lang)| ((1.0 - confidence) / f1, lang))
        .map(|(confidence, lang)| {
            let length_ratio = lengths.get(&lang.to_string()).unwrap().1 as f64 / average_length;
            let confidence = confidence * length_ratio;
            (*lang, confidence)
        })
        .collect();

    adjusted_ratios.sort_by_key(|(_, c)| (c * 1000000.0) as i32);
    //println!("{:#?}", adjusted_ratios);

    adjusted_ratios
}
