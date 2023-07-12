use std::time::Instant;

use chess::BoardStatus;
use rand::seq::IteratorRandom;

use chess::Board;
use chess::ChessMove;
use chess::Game;
use chess::MoveGen;

struct Statistics {
    start: Instant,
    iterations: i32,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            iterations: 0,
        }
    }

    pub fn increment(&mut self) {
        self.iterations = self.iterations + 1;
    }

    pub fn stop(self) {
        let dur = Instant::now() - self.start;
        println!(
            "Run finished. Considered {} positions in {} seconds",
            self.iterations,
            dur.as_secs_f32()
        )
    }
}

#[derive(Default, Clone)]
pub struct StockWish {
    depth: i32,
}

impl StockWish {
    //
    // Returns the best next move. A return-value of None means the current player is checkmate.
    //
    pub fn best_next_move(&self, game: Game) -> Option<ChessMove> {
        let board = game.current_position();
        let moves = MoveGen::new_legal(&board);
        const DEPTH: i32 = 3;
        let mut stats = Statistics::new();

        let mut algorithm = |m: ChessMove| {
            negamax_alpha_beta(
                &board.make_move_new(m),
                &mut stats,
                DEPTH,
                i32::MIN,
                i32::MAX,
            )
        };
        // let mut algorithm = |m: ChessMove| negamax(&board.make_move_new(m), &mut stats, DEPTH);

        // Get the move that leads to the best scoring child board.
        let best_move = match game.side_to_move() {
            chess::Color::Black => moves.min_by_key(|&m| algorithm(m)),
            chess::Color::White => moves.max_by_key(|&m| algorithm(m)),
        };
        stats.stop();
        return best_move;
    }
}

//
// Return a score for a board state, using a recursive negamax strategy
//
fn negamax(board: &Board, stats: &mut Statistics, remaining_depth: i32) -> i32 {
    if remaining_depth == 0 {
        // This is a leaf node, so we evaluate
        stats.increment();
        evaluate_board(board)
    } else {
        // Evaluate children and take either min or max, depending on whose turn it is
        let child_scores = MoveGen::new_legal(board)
            .map(|m| board.make_move_new(m))
            .map(|b| negamax(&b, stats, remaining_depth - 1));
        // There may not be any valid moves, such as in the case of a checkmate. It should not happen otherwise.
        if child_scores.len() == 0 {
            return evaluate_board(board);
        }
        match board.side_to_move() {
            chess::Color::Black => child_scores.min().unwrap(),
            chess::Color::White => child_scores.max().unwrap(),
        }
    }
}

//
// Return a score for a board state, using a recursive negamax strategy, with alpha-beta pruning.
// To enable pruning, we must evaluate the board score for all nodes, not just leaf nodes. This
// costs us a few board evaluations, but the pruning makes it worth it.
//
fn negamax_alpha_beta(
    board: &Board,
    stats: &mut Statistics,
    remaining_depth: i32,
    _alpha: i32,
    _beta: i32,
) -> i32 {
    if remaining_depth == 0 {
        // This is a leaf node, so we evaluate
        stats.increment();
        evaluate_board(board)
    } else {
        let mut alpha = _alpha;
        let mut beta = _beta;
        // Evaluate children and take either min or max, depending on whose turn it is
        let child_boards = MoveGen::new_legal(board).map(|m| board.make_move_new(m));
        // There may not be any valid moves, such as in the case of a checkmate. It should not happen otherwise.
        if child_boards.len() == 0 {
            return evaluate_board(board);
        }
        let mut best_value = match board.side_to_move() {
            chess::Color::White => i32::MIN,
            chess::Color::Black => i32::MAX,
        };
        for child_board in child_boards {
            let child_score =
                negamax_alpha_beta(&child_board, stats, remaining_depth - 1, alpha, beta);
            match board.side_to_move() {
                // Maximizing player
                chess::Color::White => {
                    best_value = std::cmp::max(best_value, child_score);
                    if beta < best_value {
                        break;
                    }
                    alpha = std::cmp::max(alpha, best_value);
                }
                // Minimizing player
                chess::Color::Black => {
                    best_value = std::cmp::min(best_value, child_score);
                    if best_value < alpha {
                        break;
                    }
                    beta = std::cmp::min(beta, best_value);
                }
            }
        }
        return best_value;
    }
}

// Evaluate a board state. Positive values are good for white,
// negative values are good for black.
fn evaluate_board(&board: &Board) -> i32 {
    // Checkmate, Stalemate, etc.
    let status = board.status();
    if status == BoardStatus::Checkmate {
        // If it is black to move, they are checkmated, and vice versa
        match board.side_to_move() {
            chess::Color::Black => return i32::MAX,
            chess::Color::White => return i32::MIN,
        }
    } else if status == BoardStatus::Stalemate {
        return 0;
    }

    let BASE_VALUE_MAP = |p: Option<chess::Piece>| match p {
        Some(chess::Piece::Queen) => 900,
        Some(chess::Piece::Rook) => 500,
        Some(chess::Piece::Bishop) => 320,
        Some(chess::Piece::Knight) => 300,
        Some(chess::Piece::Pawn) => 100,
        _ => 0, // The king is covered by the checkmate rules.
    };

    let white_material: i32 = board
        .color_combined(chess::Color::White)
        .map(|s| board.piece_on(s))
        .map(BASE_VALUE_MAP)
        .sum();

    let black_material: i32 = board
        .color_combined(chess::Color::Black)
        .map(|s| board.piece_on(s))
        .map(BASE_VALUE_MAP)
        .sum();

    return white_material - black_material;
}
