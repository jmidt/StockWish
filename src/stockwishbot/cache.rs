use chess::{BitBoard, ChessMove, Color};
use hashlru::Cache;

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

// A previous move evaluation
#[derive(Clone)]
pub struct CacheData {
    pub depth: i32,
    pub score: Score,
    pub targets: BitBoard,
}

pub type SWCache = Cache<u64, CacheData>;

// A collection which will retain only the N best moves, and provide a bitboard for use in move-ordering.
#[derive(Clone)]
pub struct TopTargets {
    moves: Vec<(i32, ChessMove)>,
    maximizer: bool,
    max_size: usize,
}

impl TopTargets {
    pub fn new(max_size: usize, color: Color) -> Self {
        Self {
            moves: vec![],
            maximizer: color == Color::White,
            max_size,
        }
    }

    pub fn try_insert(&mut self, score: i32, chess_move: &ChessMove) {
        if self.moves.len() < self.max_size {
            // If vector is not yet full
            self.moves.push((score, *chess_move));
        } else if self.maximizer {
            // If vector is full and high scores are preferred
            if let Some(idx) = self.moves.iter().position(|x| x.0 < score) {
                self.moves.push((score, *chess_move));
                self.moves.swap_remove(idx);
            }
        } else {
            // If vector is full and low scores are preferred
            if let Some(idx) = self.moves.iter().position(|x| x.0 > score) {
                self.moves.push((score, *chess_move));
                self.moves.swap_remove(idx);
            }
        }
    }

    pub fn just_the_moves(&self) -> impl Iterator<Item = ChessMove> + '_ {
        self.moves.iter().map(|x| x.1)
    }

    // Get a bitboard describing the target squares in the array.
    pub fn targets(&self) -> BitBoard {
        self.moves
            .iter()
            .map(|x| x.1)
            .fold(BitBoard::new(0), |acc, elem| {
                acc | BitBoard::from_square(elem.get_dest())
            })
    }
}
