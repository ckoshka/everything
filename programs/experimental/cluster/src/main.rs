use kmedoids;
use lz4_flex::compress_prepend_size;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;
use term_macros::*;
/// Takes two file objects, f1 and f2. Reads in the first 10kb of the contents of f1 and f2 as a &[u8]. Trims the buffer if the length is less than 10kb for either. Measures the total size of both of these slices combined. Compresses the slices together via compress_prepend_size. Returns the ratio of the uncompressed to the compressed size.
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
fn get_file_combinations(
    files: &Vec<std::path::PathBuf>,
) -> ndarray::Array2<f64> {
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
        for j in 0..files.len() {
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

    let matrix = matrix.into_iter().flatten().collect::<Vec<f64>>();

    let matrix: ndarray::Array2<f64> =
        ndarray::Array::from_shape_vec((files.len(), files.len()), matrix).unwrap();
    matrix
}

fn main() {
    tool! {
        args:
            - dir: String;
            - cluster_size: usize;
        ;
        body: || {
            let files = get_files(&std::path::Path::new(&dir))
                .expect("Unable to open the directory");

            let combinations = get_file_combinations(&files);

            let mut meds = kmedoids::random_initialization(
                files.len(), 
                cluster_size, 
                &mut rand::thread_rng()
            );

            let (_loss, assi, _n_iter, _): (f64, Vec<usize>, _, _) =
                kmedoids::fasterpam(
                    &combinations, 
                    &mut meds, 12000000
            );

            println!("{}, {}, {:?}", _loss, _n_iter, assi);

            let mut assi_files: Vec<(&usize, String)> = assi
                .iter()
                .zip(
                    files
                        .iter()
                        .map(|p| p.as_os_str()
                            .to_str()
                            .unwrap()
                            .replace(&dir, "")
                        )
                )
                .collect();

            assi_files.sort_by_key(|x| x.0);

            let mut mapped: HashMap<usize, Vec<String>> = HashMap::new();

            for (i, file) in assi_files.iter_mut() {
                mapped.entry(**i).or_insert(vec![]).push(file.to_string());
            }

            println!("{:#?}", mapped);
        }
    }
}
