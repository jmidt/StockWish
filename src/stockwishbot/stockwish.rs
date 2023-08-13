use chess::Board;
use chess::ChessMove;
use chess::Game;

use super::cache::CacheData;
use super::cache::SWCache;
use super::cache::Score;
use super::cache::TopTargets;
use super::evaluation::quiescent_board_score;
use super::evaluation::raw_board_score;
use super::move_ordering::generate_move_order;
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
            best_move = self.root_search(game.clone(), d);
            println!(
                "Depth: {} ::: Best move is from {} to {}",
                d,
                best_move.unwrap().get_source().to_string(),
                best_move.unwrap().get_dest().to_string()
            );
        }
        // TODO: Principal variation encounters loops in the endgame??
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

    fn root_search(&mut self, game: Game, depth: i32) -> Option<ChessMove> {
        // A special alpha-beta search function for the root node
        let mut stats = Statistics::new();
        let board = game.current_position();
        let mut alpha = i32::MIN + 1;
        let beta = i32::MAX;
        // Check cache and use for move-ordering
        let mut preferred_targets: Option<TopTargets> = None;
        if let Some(cached_evaluation) = self.cache.get(&board.get_hash()) {
            preferred_targets = Some(cached_evaluation.targets.clone());
        }
        // Prepare new cache entry
        let mut top_targets = TopTargets::new(3);
        // Time to search
        let mut best_move: Option<ChessMove> = None;
        for chess_move in generate_move_order(&board, preferred_targets) {
            let child_score: Score = -negamax_alpha_beta_cache(
                &board.make_move_new(chess_move),
                &mut stats,
                depth,
                &mut self.cache,
                -beta,
                -alpha,
                self.calibration,
            );
            let child_score_discounted = discount_checkmates(child_score.into());
            // Save if this is a good move
            top_targets.try_insert(child_score_discounted, &chess_move);
            // Check if this is the best move so far
            if child_score_discounted > alpha {
                alpha = child_score_discounted;
                best_move = Some(chess_move);
            }
        }
        self.cache.insert(
            board.get_hash(),
            CacheData {
                depth,
                score: Score::Exact(alpha),
                targets: top_targets,
            },
        );
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
            // An exact score is amazing, then we use this directly. A lower bound or upper bound potentially narrows the alpha-beta range.
            match cached_evaluation.score {
                Score::LowerBound(lower_bound) => {
                    alpha = std::cmp::max(alpha, lower_bound);
                }
                Score::UpperBound(upper_bound) => {
                    beta = std::cmp::min(beta, upper_bound);
                }
                Score::Exact(exact) => return Score::Exact(exact),
            }
        } else if cached_evaluation.depth > 0 {
            // If the depth is not enough, just use the cache for moveordering.
            // Only if depth is not zero, however. In this case, the moveordering is not helpful.
            preferred_targets = Some(cached_evaluation.targets.clone());
        }
    }
    // All valid moves in a hopefully good ordering
    let valid_moves = generate_move_order(board, preferred_targets);

    if remaining_depth <= 0 || valid_moves.is_empty() {
        stats.increment();
        // This is a leaf or terminal node, so we evaluate. We don't cache these here, since quiescent_board_score does this for us.
        //Score::Exact(raw_board_score(board, calibration)) // TODO: Change back to quiescent search
        Score::Exact(quiescent_board_score(
            board,
            cache,
            alpha,
            beta,
            calibration,
        ))
    } else {
        // Not a leaf node. We must evaluate further down.
        // First up: Null-move pruning
        // if let Some(null_moved_board) = null_move_pruning(board, remaining_depth) {
        //     // We do the null-check with a fresh cache, to not pollute the main cache.
        //     let mut null_move_cache = SWCache::new(1_000_000);
        //     let score = -negamax_alpha_beta_cache(
        //         &null_moved_board,
        //         stats,
        //         remaining_depth - 3,
        //         &mut null_move_cache,
        //         -beta,
        //         -beta + 1,
        //         calibration,
        //     );
        //     if i32::from(score) >= beta {
        //         return Score::LowerBound(score.into());
        //     }
        // }

        let mut best_value: i32 = i32::MIN;
        let mut top_targets = TopTargets::new(6);
        for chess_move in valid_moves {
            let child_score: Score = -negamax_alpha_beta_cache(
                &board.make_move_new(chess_move),
                stats,
                remaining_depth - 1,
                cache,
                -beta,
                -alpha,
                calibration,
            );
            let child_score_discounted = discount_checkmates(child_score.into());
            // Save if this is a good move
            top_targets.try_insert(child_score_discounted, &chess_move);

            best_value = std::cmp::max(best_value, child_score_discounted);
            alpha = std::cmp::max(alpha, best_value);
            if best_value >= beta {
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

fn null_move_pruning(board: &Board, remaining_depth: i32) -> Option<Board> {
    // Will return a null-moved board if it is possible to perform a null-move
    // and our heuristics allow it
    if remaining_depth < 3 {
        return None;
    }
    board.null_move()
    // Currently, we just do it all the time, but it should not be done in the endgame.
}
