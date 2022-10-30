use lz4_flex::compress_prepend_size;
use memmap::MmapOptions;
use rayon::prelude::*;
use term_macros::*;

fn rescale<T: Into<f64>, K: Send + Sync>(
    list: impl IntoParallelIterator<Item = K>,
    mapfn: impl Fn(&K) -> T,
) -> Vec<f64> {
    let items: Vec<K> = list.into_par_iter().collect();
    let avg = items.iter().map(|item| mapfn(item).into()).sum::<f64>() / items.len() as f64;
    items.iter().map(|item| mapfn(item).into() / avg).collect()
}

/// Takes a value of T.
/// Clamps it to within min and max.
fn clamp<T: std::cmp::Ord + Copy>(min: T, max: T) -> impl Fn(T) -> T {
    move |x| std::cmp::min(max, std::cmp::max(min, x))
}

/// Takes your number
/// Starts with white: (255, 255, 255)
/// Below 1? Reduces R and G values, i.e the more, the bluer.
/// Above 1? Reduces G and B values, i.e the more, the redder.
fn color<T: Into<f64> + Copy>(val: T) -> (u8, u8, u8) {
    let lgr = (255.0 * (1.0 / val.into().powf(4.0))) as u8;
    let sml = (255.0 * val.into().powf(4.0)) as u8;
    match val.into() > 1.0 {
        true => (255, lgr, lgr),
        false => (sml, 255, sml),
    }
}

/// Takes an int between 0 and 255, converts to hex
fn from_hex(n: u8) -> String {
    format!("{:x}", n)
}
/// Takes a tuple of 3 component parts, converts to a hex color.
fn rgb_hex(rgb: (u8, u8, u8)) -> String {
    let clmp = clamp(0, 255);
    format!(
        "#{}{}{}",
        from_hex(clmp(rgb.0 as u64) as u8),
        from_hex(clmp(rgb.1 as u64) as u8),
        from_hex(clmp(rgb.2 as u64) as u8) // this errors after it exceeds 255
    )
}

fn color_int<I: Into<f64> + std::marker::Copy>(i: I) -> String {
    ansi_hex_color::colored(&rgb_hex(color(i)), "#000000", &i.into().to_string())
}

/// Uses memmap to concatenate every file in a directory (recursively)
fn glob_memmap<FilterFn: Fn(&str) -> bool>(
    dir: &str,
    filterfn: &FilterFn,
) -> Result<Vec<(String, memmap::Mmap)>, std::io::Error> {
    let dir_path = std::path::PathBuf::from(dir);
    if !dir_path.is_dir() {
        if !filterfn(dir) {
            return Ok(vec![]);
        }
        return Ok(vec![(dir.to_string(), unsafe {
            MmapOptions::new().map(&std::fs::File::open(&dir_path)?)?
        })]);
    }
    let files = std::fs::read_dir(dir_path)?.map(|e| e.unwrap().path());
    Ok(files
        .map(|f| glob_memmap(f.to_str().unwrap(), filterfn))
        .filter(|e| e.is_ok())
        .map(|e| e.unwrap())
        .flatten()
        .collect())
}

fn main() {

    tool! {
        args:
            - only_include: String;
            - directory: String = "\t".to_string();
        ;

        body: || {
            let only_include_ref = &only_include;
            let filter = |s: &str| s.contains(only_include_ref);
            let named_mms = glob_memmap(&directory, &filter).unwrap();
            let ratios = named_mms.par_iter().map(|(_, mmap)| {
                let size_before = mmap[..].len();
                let size_after = compress_prepend_size(&mmap[..]).len();
                let ratio = size_before as f64 / size_after as f64;
                ratio
            });
            let mut ratios_adj = rescale(ratios, |x| *x)
                .into_iter()
                .zip(named_mms.iter().map(|(name, _)| name))
                .collect::<Vec<_>>();
            ratios_adj.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for (sc, name) in ratios_adj.into_iter() {
                println!("{}", name.replace(&directory, ""));
                println!("{}", color_int(sc));
            }
        }
    }
}
