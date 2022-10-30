use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use rayon::prelude::*;

pub type SimilarityMatrix = Vec<Vec<f64>>;
pub type ClusterId = usize;

#[derive(Hash, Clone, PartialEq, Eq)]
pub struct Language {
    pub id: usize,
}

pub struct Cluster {
    pub languages: Arc<Mutex<HashSet<Language>>>,
}

pub struct ClusterManager {
    pub clusters: Vec<Cluster>,
    matrix: SimilarityMatrix,
}

pub fn vec_into_clusters(langs: impl IntoIterator<Item = Language>) -> Vec<Cluster> {
    langs
        .into_iter()
        .map(|lang| Cluster {
            languages: Arc::new(Mutex::new(vec![lang].into_iter().collect::<HashSet<_>>())),
        })
        .collect()
}

// pretty much done, now just need to do the language mappings
// then change the return type in main.rs
// then map the results back to their language names
// print them, done

impl ClusterManager {
    pub fn new(clusters: Vec<Cluster>, matrix: SimilarityMatrix) -> Self {
        ClusterManager { clusters, matrix }
    }
    fn choose_worst(&self, cluster_id: usize) -> Option<Language> {
        let mut languages = self.clusters[cluster_id].languages.lock().unwrap();
        if languages.len() == 0 {
            return None;
        }
        let result = languages
            .iter()
            .map(|lang| {
                (
                    lang.id,
                    languages
                        .iter()
                        .map(|other_lang| self.matrix[lang.id][other_lang.id])
                        .sum::<f64>()
                        / languages.len() as f64,
                )
            })
            .max_by(|(_, how_good), (_, how_good_2)| how_good.partial_cmp(how_good_2).unwrap())
            .map(|bad_lang| {
                languages.retain(|l| l.id != bad_lang.0);
                Language { id: bad_lang.0 }
            });
        result
    }

    // if none, that means it's an empty cluster
    // what's the bet that it assigns them all into one huge cluster?

    fn choose_new_home(&self, language: &Language, original_cluster: ClusterId) -> ClusterId {
        let best_home = self
            .clusters
            .iter()
            .filter(|c| c.languages.lock().unwrap().len() > 0)
            .enumerate()
            .map(|(i, cluster)| {
                let cluster_lock = cluster.languages.lock().unwrap();
                (
                    i,
                    cluster_lock
                        .iter()
                        .map(|lang| self.matrix[language.id][lang.id])
                        .sum::<f64>()
                        / cluster_lock.len() as f64,
                )
            }) // whether total inter-group harmony would be increased or decreased by adding that language,
            // is the determining factor for the language being added to an empty cluster
            .min_by(|(_, how_good), (_, how_good_2)| {
                how_good
                    .partial_cmp(how_good_2)
                    .unwrap_or_else(|| std::cmp::Ordering::Equal)
            });
        let best_cluster_id = best_home.unwrap().0;
        if best_cluster_id == original_cluster {
            return self.clusters.iter().enumerate().filter(|(_, c)| c.languages.lock().unwrap().is_empty()).map(|(i, _)| i).next().unwrap();
        }
        best_cluster_id
    }

    fn add_lang_to_cluster(&self, cluster_id: usize, language: Language) {
        self.clusters[cluster_id].languages.lock().unwrap().insert(language);
    }

    pub fn run(&self, num_iters: usize, callback: Option<&(dyn Fn(usize) + Send + Sync)>) {
        //let mut curr_id = 0;
        // could just go "consider only these ones"
        (0..4).into_par_iter().for_each(|thread_no| {
            let starting_position = thread_no * (num_iters / 4);
            for i in starting_position..(num_iters+starting_position) {
                let curr_id = (i as i32).rem_euclid(self.clusters.len() as i32) as usize;
                let result = self.choose_worst(curr_id).map(|lang| {
                    let new_home = self.choose_new_home(&lang, curr_id);
                    self.add_lang_to_cluster(new_home, lang)
                });
                callback.map(|cb| cb(i - starting_position));
                //if result.is_none() {
                //    self.clusters.remove(curr_id);
                //}
            }
        });
    }
}
// if the best one is the same as the original id, then put it in a new category?