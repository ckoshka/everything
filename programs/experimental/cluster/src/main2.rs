use term_macros::*;
use lz4_flex::compress_prepend_size;
use rayon::prelude::*;
use std::{collections::HashMap, io::Read};

fn compression_ratio(
    f1: &[u8],
    f2: &[u8],
    compr_f1_len: usize,
    compr_f2_len: usize,
) -> Result<f64, std::io::Error> {
    let together = f1.into_iter().chain(f2).copied().collect::<Vec<u8>>();
    let total_length = together.len();
    if total_length == 0 || f1.len() == 0 || f2.len() == 0 {
        return Ok(1.0);
    }
    let compressed_together = compress_prepend_size(&together);

    let expected = (compr_f1_len + compr_f2_len) as f64 / (f1.len() + f2.len()) as f64;
    let actual = (compressed_together.len()) as f64 / (f1.len() + f2.len()) as f64;
    Ok(actual / expected)
}
/// Returns a list of all files found in the top level of a directory (and not within any subdirectories). Ignores folders.
fn get_files(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let files = std::fs::read_dir(path)?
        .into_iter()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                Some(path)
            } else {
                None
            }
        })
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    Ok(files)
}

/// Takes a Vec<std::path::PathBuf> and returns a matrix of the compression ratio of each file in the list with each other file.
fn get_file_combinations(files: &Vec<std::path::PathBuf>) -> Vec<Vec<f64>> {
    let file_slices = files
        .into_par_iter()
        .map(|file| {
            let file = std::fs::File::open(file).unwrap();
            file.bytes()
                .take(200_000)
                .map(|res| res.unwrap())
                .collect::<Vec<u8>>()
        })
        .collect::<Vec<Vec<_>>>();

    let file_slice_compressed_lengths = file_slices
        .par_iter()
        .map(|file_slice| {
            let compressed_file_slice = compress_prepend_size(&file_slice);
            compressed_file_slice.len()
        })
        .collect::<Vec<_>>();

    let mut matrix: Vec<Vec<f64>> = vec![vec![0.0; files.len()]; files.len()];

    matrix.par_iter_mut().enumerate().for_each(|(i, row)| {
        for j in 0..files.len() { //????
            if j != i {
                let result = compression_ratio(
                    &file_slices[i],
                    &file_slices[j],
                    file_slice_compressed_lengths[i],
                    file_slice_compressed_lengths[j],
                )
                .unwrap();
                row[j] = result;
            }
        }
    });

    matrix
}

fn main() {
    tool! {
        args:
            - dir: String;
            - num_iters: usize = 100000;
        ;
        body: || {
            let files = get_files(&std::path::Path::new(&dir))
                .expect("Unable to open the directory");

            let matrix = get_file_combinations(&files);

            let languages = (0..matrix.len()).map(|id| {
                Language {
                    id
                }
            });

            let clusters = vec_into_clusters(languages);

            let manager = ClusterManager::new(clusters, matrix);

            manager.run(num_iters, Some(&|i| {
                if i % (num_iters / 10) == 0 {
                    println!("Currently on generation {}", i);
                }
            }));

            manager.clusters.iter().enumerate().for_each(|(id, cluster)| {
                if cluster.languages.lock().unwrap().len() == 0 {
                    return;
                }
                println!("\n{}", id);
                cluster.languages.lock().unwrap().iter().for_each(|lang| {
                    files[lang.id].as_os_str().to_str().map(|string| {
                        println!("{}", string);
                    });
                });
            });
        }
    }
}
