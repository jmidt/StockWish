use chess::BitBoard;
use chess::EMPTY;

use chess::Board;
use chess::ChessMove;
use chess::Game;
use chess::MoveGen;

use super::cache::CacheData;
use super::cache::SWCache;
use super::cache::Score;
use super::cache::TopTargets;
use super::evaluation::quiescent_board_score;
use super::evaluation::raw_board_score;
use super::statistics::Statistics;

#[derive(Default, Clone, Copy)]
pub struct Calibration {
    pub positional_weight: i32,
}

// TODO: Should not derive clone, since it now owns a lot of data.
#[derive(Clone)]
pub struct StockWish {
    depth: i32,
    cache: SWCache,
    calibration: Calibration,
}

impl Default for StockWish {
    fn default() -> Self {
        Self {
            depth: 8,
            cache: SWCache::new(10_000_000),
            calibration: Calibration::default(),
        }
    }
}

impl StockWish {
    pub fn new(depth: i32, calibration: Calibration) -> Self {
        Self {
            depth,
            cache: SWCache::new(10_000_000),
            calibration,
        }
    }

    //
    // Returns the best next move using iterative deepening.
    //
    pub fn best_next_move_iterative_deepening(&mut self, game: Game) -> Option<ChessMove> {
        let iterative_deepening_depths = vec![4, 6, 8];
        let mut best_move = None;
        for d in iterative_deepening_depths {
            best_move = self.best_next_move_at_depth(game.clone(), d);
        }
        println!(
            "Best move is from {} to {}",
            best_move.unwrap().get_source().to_string(),
            best_move.unwrap().get_dest().to_string()
        );
        best_move
    }
    //
    // Returns the best next move. A return-value of None means the current player is checkmate.
    //
    pub fn best_next_move(&mut self, game: Game) -> Option<ChessMove> {
        self.best_next_move_at_depth(game, self.depth)
    }

    fn best_next_move_at_depth(&mut self, game: Game, depth: i32) -> Option<ChessMove> {
        let board = game.current_position();
        let moves = MoveOrder::new(&board);
        let mut stats = Statistics::new();

        let mut algorithm = |m: ChessMove| {
            negamax_alpha_beta_cache(
                &board.make_move_new(m),
                &mut stats,
                depth,
                &mut self.cache,
                i32::MIN,
                i32::MAX,
                self.calibration,
            )
        };
        // Get the move that leads to the best scoring child board.
        let best_move = match game.side_to_move() {
            chess::Color::White => moves.max_by_key(|&m| -> i32 { algorithm(m).into() }),
            chess::Color::Black => moves.min_by_key(|&m| -> i32 { algorithm(m).into() }),
        };
        stats.stop();
        best_move
    }
}

fn negamax_alpha_beta_cache(
    board: &Board,
    stats: &mut Statistics,
    remaining_depth: i32,
    cache: &mut SWCache,
    _alpha: i32,
    _beta: i32,
    calibration: Calibration,
) -> Score {
    let mut preferred_targets: Option<BitBoard> = None;
    let mut alpha = _alpha;
    let mut beta = _beta;
    // Check cache
    if let Some(cached_evaluation) = cache.get(&board.get_hash()) {
        if cached_evaluation.depth >= remaining_depth {
            // If this move exists in the cache at a depth of at least remaining_depth, use this.
            // An exact score is amazing, then we use this directly. A lower bound or upper bound narrows the alpha-beta range.
            match cached_evaluation.score {
                Score::LowerBound(lower_bound) => alpha = lower_bound,
                Score::UpperBound(upper_bound) => beta = upper_bound,
                Score::Exact(exact) => return Score::Exact(exact),
            }
        } else {
            // If the depth is not enough, just use the cache for moveordering
            preferred_targets = Some(cached_evaluation.targets);
        }
    }
    // All valid moves
    let valid_moves = if let Some(t) = preferred_targets {
        MoveOrder::new_from_preferred_targets(board, t)
    } else {
        MoveOrder::new(board)
    };
    if remaining_depth == 0 || valid_moves.len() == 0 {
        // This is a leaf or terminal node, so we evaluate. We don't cache these here, since quiescent_board_score does this for us.
        stats.increment();
        Score::Exact(raw_board_score(board, calibration)) // TODO: Change to quiescent search
    } else {
        let mut best_value: i32 = match board.side_to_move() {
            chess::Color::White => i32::MIN,
            chess::Color::Black => i32::MAX,
        };
        let mut top_targets = TopTargets::new(5, board.side_to_move());
        for chess_move in valid_moves {
            let child_board = board.make_move_new(chess_move);
            let child_score: i32 = negamax_alpha_beta_cache(
                &child_board,
                stats,
                remaining_depth - 1,
                cache,
                alpha,
                beta,
                calibration,
            )
            .into();
            top_targets.try_insert(child_score, &chess_move);
            match board.side_to_move() {
                // Maximizing player
                chess::Color::White => {
                    best_value = std::cmp::max(best_value, child_score);
                    alpha = std::cmp::max(alpha, best_value);
                    if beta < best_value {
                        let score = Score::LowerBound(best_value);
                        cache.insert(
                            board.get_hash(),
                            CacheData {
                                depth: remaining_depth,
                                score,
                                targets: top_targets.targets(),
                            },
                        );
                        return score;
                    }
                }
                // Minimizing player
                chess::Color::Black => {
                    best_value = std::cmp::min(best_value, child_score);
                    beta = std::cmp::min(beta, best_value);
                    if best_value < alpha {
                        let score = Score::UpperBound(best_value);
                        cache.insert(
                            board.get_hash(),
                            CacheData {
                                depth: remaining_depth,
                                score,
                                targets: top_targets.targets(),
                            },
                        );
                        return score;
                    }
                }
            }
        }
        let score = Score::Exact(best_value);
        cache.insert(
            board.get_hash(),
            CacheData {
                depth: remaining_depth,
                score,
                targets: top_targets.targets(),
            },
        );
        score
    }
}

//
// A better move order for iteration, hitting potentially high-value moves earlier
//
enum MoveOrderStage {
    Hints,
    Captures,
    Remaining,
}

struct MoveOrder {
    movegen: MoveGen,
    board: Board,
    stage: MoveOrderStage,
}

impl MoveOrder {
    fn movegen_from_mask(board: &Board, mask: BitBoard) -> MoveGen {
        let mut movegen = MoveGen::new_legal(board);
        movegen.set_iterator_mask(mask);
        movegen
    }

    pub fn new_from_preferred_targets(board: &Board, targets: BitBoard) -> Self {
        // Construct a MoveOrder in the `Hints` stage.
        Self {
            movegen: Self::movegen_from_mask(board, targets),
            board: *board,
            stage: MoveOrderStage::Hints,
        }
    }

    pub fn new(board: &Board) -> Self {
        // Construct a MoveOrder in the `Captures` stage.
        Self {
            movegen: Self::movegen_from_mask(board, *board.color_combined(!board.side_to_move())),
            board: *board,
            stage: MoveOrderStage::Captures,
        }
    }
}

impl ExactSizeIterator for MoveOrder {
    /// Give the exact length of this iterator
    fn len(&self) -> usize {
        self.movegen.len()
    }
}

impl Iterator for MoveOrder {
    type Item = ChessMove;

    // TODO: Refactor this
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.movegen.next();
        // First, iterate through capturing moves.
        match self.stage {
            MoveOrderStage::Hints => {
                if next.is_none() {
                    self.movegen
                        .set_iterator_mask(*self.board.color_combined(self.board.side_to_move()));
                    self.stage = MoveOrderStage::Captures;
                    self.movegen.next()
                } else {
                    next
                }
            }
            MoveOrderStage::Captures => {
                if next.is_none() {
                    self.movegen.set_iterator_mask(!EMPTY);
                    self.stage = MoveOrderStage::Remaining;
                    self.movegen.next()
                } else {
                    next
                }
            }
            MoveOrderStage::Remaining => next,
        }
    }
}