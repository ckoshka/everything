use std::collections::HashMap;

pub type SentenceId = u64;
pub type Position = usize;
pub type SortPositions = HashMap<SentenceId, Position>;
