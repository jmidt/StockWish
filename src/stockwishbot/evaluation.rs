// Evaluation of a board state. Usually used for leaf nodes in the game tree. Positive values are good for white,
// negative values are good for black.

use chess::BitBoard;
use chess::Board;
use chess::BoardStatus;
use chess::MoveGen;

use super::cache::CacheData;
use super::cache::SWCache;
use super::cache::Score;

pub fn quiescent_board_score(board: &Board, cache: &mut SWCache) -> i32 {
    // Evaluate a board. We only actually evaluate quiescent board states, so we run through
    // a new game tree, with no max depth, only considering captures.
    // TODO: Currently using alpha-beta pruning, but I hear delta-pruning is good at this?
    let score = quiescent_alpha_beta(board, cache, i32::MIN, i32::MAX);
    // TODO: We could potentially find some good targets, but it would only involve captures,
    // so probably not so useful for general tree search.
    cache.insert(
        board.get_hash(),
        CacheData {
            depth: 0,
            score,
            targets: BitBoard::new(0),
        },
    );
    score.into()
}

fn quiescent_alpha_beta(board: &Board, cache: &mut SWCache, _alpha: i32, _beta: i32) -> Score {
    let mut alpha = _alpha;
    let mut beta = _beta;
    // Evaluate captures and take either min or max, depending on whose turn it is
    let mut captures = MoveGen::new_legal(board);
    captures.set_iterator_mask(*board.color_combined(!board.side_to_move()));

    let mut num_captures = 0;
    let mut best_value: i32 = match board.side_to_move() {
        chess::Color::White => i32::MIN,
        chess::Color::Black => i32::MAX,
    };
    for capture in captures {
        num_captures += 1;
        let child_board = board.make_move_new(capture);
        let child_score: i32 = quiescent_alpha_beta(&child_board, cache, alpha, beta).into();
        match board.side_to_move() {
            // Maximizing player
            chess::Color::White => {
                best_value = std::cmp::max(best_value, child_score);
                alpha = std::cmp::max(alpha, best_value);
                if beta < best_value {
                    return Score::LowerBound(best_value);
                }
            }
            // Minimizing player
            chess::Color::Black => {
                best_value = std::cmp::min(best_value, child_score);
                beta = std::cmp::min(beta, best_value);
                if best_value < alpha {
                    return Score::UpperBound(best_value);
                }
            }
        }
    }
    // If there were no captures, we have hit a quiescent board state and we evaluate.
    if num_captures == 0 {
        return Score::Exact(raw_board_score(board));
    }
    // If we get here, we HAD possible captures to do, and return the score for the optimal capture.
    Score::Exact(best_value)
}

pub fn raw_board_score(board: &Board) -> i32 {
    // Checkmate, Stalemate, etc.
    match board.status() {
        BoardStatus::Checkmate => {
            // If it is black to move, they are checkmated, and vice versa
            match board.side_to_move() {
                chess::Color::Black => return i32::MAX,
                chess::Color::White => return i32::MIN,
            }
        }
        BoardStatus::Stalemate => {
            return 0;
        }
        _ => {
            return material_balance(board);
        }
    }
}

fn material_balance(board: &Board) -> i32 {
    let white_material: i32 = board
        .color_combined(chess::Color::White)
        .map(|s| board.piece_on(s))
        .map(piece_value)
        .sum();

    let black_material: i32 = board
        .color_combined(chess::Color::Black)
        .map(|s| board.piece_on(s))
        .map(piece_value)
        .sum();

    // The board evaluation is currently just the material balance.
    white_material - black_material
}

#[inline(always)]
fn piece_value(p: Option<chess::Piece>) -> i32 {
    match p {
        Some(chess::Piece::Queen) => 900,
        Some(chess::Piece::Rook) => 500,
        Some(chess::Piece::Bishop) => 310,
        Some(chess::Piece::Knight) => 300,
        Some(chess::Piece::Pawn) => 100,
        _ => 0, // The king is covered by the checkmate rules.
    }
}
