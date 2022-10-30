use crate::{Sorter, Translation};
use rayon::prelude::*;

pub fn sort<'a>(
    sorters: Vec<Box<dyn Sorter + Send + Sync>>,
    txs: &'a mut Vec<Translation>,
) -> &'a [Translation] {
    let sorts: Vec<_> = sorters.par_iter().map(|s| s.sort(txs.as_slice())).collect();
    let txs_slice = &mut txs[..];
    txs_slice.par_sort_by_cached_key(|tx| {
        sorts
            .iter()
            .map(|map| *map.get(&tx.id).unwrap())
            .sum::<usize>()
    });
    txs_slice
}
