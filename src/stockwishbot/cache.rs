use chess::{Board, ChessMove};
use hashlru::Cache;
use itertools::Itertools;
use std::ops::Neg;

#[derive(Copy, Clone)]
pub enum Score {
    Exact(i32),
    UpperBound(i32),
    LowerBound(i32),
}

impl From<Score> for i32 {
    fn from(score: Score) -> i32 {
        match score {
            Score::Exact(val) => val,
            Score::UpperBound(val) => val,
            Score::LowerBound(val) => val,
        }
    }
}

impl Neg for Score {
    type Output = Score;

    fn neg(self) -> Self::Output {
        match self {
            Score::Exact(val) => Score::Exact(-val),
            Score::UpperBound(val) => Score::LowerBound(-val),
            Score::LowerBound(val) => Score::UpperBound(-val),
        }
    }
}

// A previous move evaluation
#[derive(Clone)]
pub struct CacheData {
    pub depth: i32,
    pub score: Score,
    pub targets: TopTargets,
}

pub type SWCache = Cache<u64, CacheData>;

pub fn insert_in_cache_if_better(
    board: &Board,
    depth: i32,
    score: &Score,
    targets: TopTargets,
    cache: &mut SWCache,
) {
    let hash = board.get_hash();
    if let Some(cached) = cache.get(&hash) {
        if depth > cached.depth {
            cache.insert(
                board.get_hash(),
                CacheData {
                    depth,
                    score: *score,
                    targets,
                },
            );
        }
    } else {
        cache.insert(
            board.get_hash(),
            CacheData {
                depth,
                score: *score,
                targets,
            },
        );
    }
}

// A collection which will retain only the N best moves, and provide a bitboard for use in move-ordering.
#[derive(Clone)]
pub struct TopTargets {
    pub moves: Vec<(i32, ChessMove)>,
    max_size: usize,
}

impl TopTargets {
    pub fn new(max_size: usize) -> Self {
        Self {
            moves: vec![],
            max_size,
        }
    }

    pub fn try_insert(&mut self, score: i32, chess_move: &ChessMove) {
        if self.moves.len() < self.max_size {
            // If vector is not yet full
            self.moves.push((score, *chess_move));
        } else {
            // If vector is full, high scores are preferred
            if let Some(idx) = self.moves.iter().position(|x| x.0 < score) {
                self.moves.push((score, *chess_move));
                self.moves.swap_remove(idx);
            }
        }
    }

    pub fn ordered_moves(&self) -> Vec<ChessMove> {
        // Used to efficiently put into a Vec and pop from
        self.moves
            .iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .map(|x| x.1)
            .collect_vec()
    }
}
