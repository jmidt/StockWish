use std::ptr::null;

use chess::Board;
use chess::ChessMove;
use chess::Game;

use super::cache::CacheData;
use super::cache::SWCache;
use super::cache::Score;
use super::cache::TopTargets;
use super::evaluation::quiescent_board_score;
use super::evaluation::raw_board_score;
use super::move_ordering::MoveOrder;
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
        let iterative_deepening_depths = vec![1, 2, 3, 4, 5, 6];
        let mut best_move = None;
        println!("--------------------");
        for d in iterative_deepening_depths {
            best_move = self.best_next_move_at_depth(game.clone(), d);
            println!(
                "Depth: {} ::: Best move is from {} to {}",
                d,
                best_move.unwrap().get_source().to_string(),
                best_move.unwrap().get_dest().to_string()
            );
        }
        // TODO: Principal variation ecounters loops in the endgame??
        // if let Some(first_move) = best_move {
        //     println!(
        //         "Principal variation is {:?}",
        //         self.get_principal_variation(game.current_position(), first_move)
        //             .iter()
        //             .map(|m| m.to_string())
        //             .reduce(|acc, m| acc + ", " + &m)
        //             .unwrap()
        //     );
        // }
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

    // Reconstructs the principal variation from the cache
    fn get_principal_variation(
        &mut self,
        current_board: Board,
        first_move: ChessMove,
    ) -> Vec<ChessMove> {
        let mut pv = vec![first_move];
        let mut board = current_board.make_move_new(first_move);
        while let Some(cached) = self.cache.get(&board.get_hash()) {
            if let Some(next_move) = cached.targets.ordered_moves().last() {
                pv.push(*next_move);
                board = board.make_move_new(*next_move);
            } else {
                break;
            }
        }
        pv
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
    let mut preferred_targets: Option<TopTargets> = None;
    let mut alpha = _alpha;
    let mut beta = _beta;
    // Check cache
    if let Some(cached_evaluation) = cache.get(&board.get_hash()) {
        if cached_evaluation.depth >= remaining_depth {
            // If this move exists in the cache at a depth of at least remaining_depth, use this.
            // An exact score is amazing, then we use this directly. A lower bound or upper bound narrows the alpha-beta range.
            match cached_evaluation.score {
                Score::LowerBound(lower_bound) => {
                    alpha = lower_bound;
                }
                Score::UpperBound(upper_bound) => {
                    beta = upper_bound;
                }
                Score::Exact(exact) => return Score::Exact(exact),
            }
        } else {
            // If the depth is not enough, just use the cache for moveordering
            preferred_targets = Some(cached_evaluation.targets.clone());
        }
    }
    // All valid moves
    let valid_moves = if let Some(t) = preferred_targets {
        MoveOrder::new_with_hint(board, t)
    } else {
        MoveOrder::new(board)
    };
    if remaining_depth <= 0 || valid_moves.len() == 0 {
        stats.increment();
        // This is a leaf or terminal node, so we evaluate. We don't cache these here, since quiescent_board_score does this for us.
        // Score::Exact(raw_board_score(board, calibration)) // TODO: Change back to quiescent search
        let score = Score::Exact(raw_board_score(board, calibration));
        cache.insert(
            board.get_hash(),
            CacheData {
                depth: remaining_depth,
                score,
                targets: TopTargets::new(0, chess::Color::White),
            },
        );
        score
    } else {
        // Not a leaf node. We must evaluate further down.
        // First up: Null-move pruning (TODO: After proper negamax)
        // if let Some(null_moved_board) = null_move_pruning(board) {
        //     match negamax_alpha_beta_cache(
        //         &null_moved_board,
        //         stats,
        //         remaining_depth - 2,
        //         cache,
        //         alpha,
        //         beta,
        //         calibration,
        //     ) {
        //         // For exact scores, we are still on the right track, so we continue with the proper evaluation
        //         Score::Exact(_) => {}
        //         // For everything other than exact scores (meaning there was a cutoff), we return.
        //         x => return x,
        //     }
        // }

        let mut best_value: i32 = match board.side_to_move() {
            chess::Color::White => i32::MIN,
            chess::Color::Black => i32::MAX,
        };
        let mut top_targets = TopTargets::new(10, board.side_to_move());
        for chess_move in valid_moves {
            let child_board = board.make_move_new(chess_move);
            let child_score: i32 = discount_checkmates(
                negamax_alpha_beta_cache(
                    &child_board,
                    stats,
                    remaining_depth - 1,
                    cache,
                    alpha,
                    beta,
                    calibration,
                )
                .into(),
            );
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
                                targets: top_targets,
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
                                targets: top_targets,
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
                targets: top_targets,
            },
        );
        score
    }
}

fn discount_checkmates(score: i32) -> i32 {
    // If score is very close to the CHECKMATE scores, discount by 1 (towards 0).
    // This ensures shorter checkmate lines are preferred.
    const THRESHOLD: i32 = 100;
    if score < i32::MIN + THRESHOLD {
        score + 1
    } else if score > i32::MAX - THRESHOLD {
        score - 1
    } else {
        score
    }
}

fn null_move_pruning(board: &Board) -> Option<Board> {
    // Will return a null-moved board if it is possible to perform a null-move
    // and our heuristics allow it
    board.null_move()
    // Currently, we just do it all the time, but it should not be done in the endgame.
}
