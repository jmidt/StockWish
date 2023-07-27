// Evaluation of a board state. Usually used for leaf nodes in the game tree. Positive values are good for white,
// negative values are good for black.

use std::str::FromStr;

use chess::BitBoard;
use chess::Board;
use chess::BoardStatus;
use chess::Game;
use chess::MoveGen;
use chess::Square;
use chess::ALL_SQUARES;

use super::cache::CacheData;
use super::cache::SWCache;
use super::cache::Score;
use super::cache::TopTargets;
use super::Calibration;

pub fn quiescent_board_score(board: &Board, cache: &mut SWCache, calibration: Calibration) -> i32 {
    // Evaluate a board. We only actually evaluate quiescent board states, so we run through
    // a new game tree, with no max depth, only considering captures.
    // TODO: Currently using alpha-beta pruning, but I hear delta-pruning is good at this?
    let score = quiescent_alpha_beta(board, cache, i32::MIN, i32::MAX, calibration);
    // TODO: We could potentially find some good targets, but it would only involve captures,
    // so probably not so useful for general tree search.
    cache.insert(
        board.get_hash(),
        CacheData {
            depth: 0,
            score,
            targets: TopTargets::new(5, board.side_to_move()),
        },
    );
    score.into()
}

fn quiescent_alpha_beta(
    board: &Board,
    cache: &mut SWCache,
    _alpha: i32,
    _beta: i32,
    calibration: Calibration,
) -> Score {
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
        let child_score: i32 =
            quiescent_alpha_beta(&child_board, cache, alpha, beta, calibration).into();
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
        return Score::Exact(raw_board_score(board, calibration));
    }
    // If we get here, we HAD possible captures to do, and return the score for the optimal capture.
    Score::Exact(best_value)
}

pub fn raw_board_score(board: &Board, calibration: Calibration) -> i32 {
    // Checkmate, Stalemate, etc.
    match board.status() {
        BoardStatus::Checkmate => {
            // If it is black to move, they are checkmated, and vice versa
            match board.side_to_move() {
                chess::Color::White => i32::MIN,
                chess::Color::Black => i32::MAX,
            }
        }
        BoardStatus::Stalemate => 0,
        _ => ongoing_raw_board_score(board, calibration),
    }
}

fn ongoing_raw_board_score(board: &Board, calibration: Calibration) -> i32 {
    let material = material_balance(board);
    // let mobility = mobility_score(board);
    let positional = singlet_positions(board);
    return 12 * material + 1 * positional;
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

// TODO: Wait until we only search quiescent positions (no checks)
// fn mobility_score(board: &Board) -> i32 {
//     let current_player_mobility = MoveGen::new_legal(board).len();
//     let opposing_player_mobility = if let Some(reversed_board) = board.null_move() {
//         MoveGen::new_legal(&reversed_board).len()
//     } else {
//         // Current player is in check.
//         let all_checkers = |b: &Board| b.checkers()
//         let board_without_checkers = board
//     }

//     current_player_mobility as i32 - opposing_player_mobility as i32
// }

enum GamePhase {
    Opening,
    MiddleGame,
    Endgame,
}

fn game_phase(board: &Board) -> GamePhase {
    let total_material: i32 = ALL_SQUARES
        .map(|s| board.piece_on(s))
        .map(piece_value)
        .into_iter()
        .sum();
    if total_material > 6800 {
        GamePhase::Opening
    } else if total_material > 3000 {
        GamePhase::MiddleGame
    } else {
        GamePhase::Endgame
    }
}

#[inline(always)]
fn piece_value(p: Option<chess::Piece>) -> i32 {
    match p {
        Some(chess::Piece::Queen) => 900,
        Some(chess::Piece::Rook) => 500,
        Some(chess::Piece::Bishop) => 330,
        Some(chess::Piece::Knight) => 320,
        Some(chess::Piece::Pawn) => 100,
        _ => 0, // The king is covered by the checkmate rules.
    }
}

struct HeatMap {
    cells: [i32; 64], // Row-major: a1, b1, c1, ...
}

impl HeatMap {
    pub const fn new(vals: [i32; 64]) -> Self {
        Self { cells: vals }
    }

    pub fn dot(&self, bitboard: &BitBoard) -> i32 {
        bitboard
            .into_iter()
            .map(|sq| self.cells[63 - sq.to_index()])
            .sum()
    }

    pub fn mirror_copy(&self) -> Self {
        let mut vals = Vec::new();
        for chunk in self.cells.chunks_exact(8).rev() {
            vals.extend_from_slice(chunk);
        }
        Self::new(vals.try_into().unwrap())
    }
}

fn singlet_positions(board: &Board) -> i32 {
    let BLACK_PAWN: HeatMap = WHITE_PAWN.mirror_copy();
    let BLACK_KNIGHT: HeatMap = WHITE_KNIGHT.mirror_copy();
    let BLACK_BISHOP: HeatMap = WHITE_BISHOP.mirror_copy();
    let BLACK_ROOK: HeatMap = WHITE_ROOK.mirror_copy();
    let BLACK_QUEEN: HeatMap = WHITE_QUEEN.mirror_copy();
    let BLACK_KING_OPENING: HeatMap = WHITE_KING_OPENING.mirror_copy();
    let BLACK_KING_ENDGAME: HeatMap = WHITE_KING_ENDGAME.mirror_copy();

    let white_pawns = board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Pawn);
    let black_pawns = board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Pawn);
    let white_knights =
        board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Knight);
    let black_knights =
        board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Knight);
    let white_bishops =
        board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Bishop);
    let black_bishops =
        board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Bishop);
    let white_rooks = board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Rook);
    let black_rooks = board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Rook);
    let white_queens =
        board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Queen);
    let black_queens =
        board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Queen);
    let white_kings = board.color_combined(chess::Color::White) & board.pieces(chess::Piece::King);
    let black_kings = board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::King);

    WHITE_PAWN.dot(&white_pawns) - BLACK_PAWN.dot(&black_pawns) + WHITE_KNIGHT.dot(&white_knights)
        - BLACK_KNIGHT.dot(&black_knights)
        + WHITE_BISHOP.dot(&white_bishops)
        - BLACK_BISHOP.dot(&black_bishops)
        + WHITE_ROOK.dot(&white_rooks)
        - BLACK_ROOK.dot(&black_rooks)
        + WHITE_QUEEN.dot(&white_queens)
        - BLACK_QUEEN.dot(&black_queens)
        + WHITE_KING_OPENING.dot(&white_kings)
        - BLACK_KING_OPENING.dot(&black_kings)
}

// Pawns
const WHITE_PAWN: HeatMap = HeatMap::new([
    0, 0, 0, 0, 0, 0, 0, 0, //
    50, 50, 50, 50, 50, 50, 50, 50, //
    10, 10, 20, 30, 30, 20, 10, 10, //
    5, 5, 10, 25, 25, 10, 5, 5, //
    0, 0, 0, 20, 20, 0, 0, 0, //
    5, -5, -10, 0, 0, -10, -5, 5, //
    5, 10, 10, -20, -20, 10, 10, 5, //
    0, 0, 0, 0, 0, 0, 0, 0, //
]);
const WHITE_KNIGHT: HeatMap = HeatMap::new([
    -50, -40, -30, -30, -30, -30, -40, -50, //
    -40, -20, 0, 0, 0, 0, -20, -40, //
    -30, 0, 10, 15, 15, 10, 0, -30, //
    -30, 5, 15, 20, 20, 15, 5, -30, //
    -30, 0, 15, 20, 20, 15, 0, -30, //
    -30, 5, 10, 15, 15, 10, 5, -30, //
    -40, -20, 0, 5, 5, 0, -20, -40, //
    -50, -40, -30, -30, -30, -30, -40, -50, //
]);
const WHITE_BISHOP: HeatMap = HeatMap::new([
    -20, -10, -10, -10, -10, -10, -10, -20, //
    -10, 0, 0, 0, 0, 0, 0, -10, //
    -10, 0, 5, 10, 10, 5, 0, -10, //
    -10, 5, 5, 10, 10, 5, 5, -10, //
    -10, 0, 10, 10, 10, 10, 0, -10, //
    -10, 10, 10, 10, 10, 10, 10, -10, //
    -10, 5, 0, 0, 0, 0, 5, -10, //
    -20, -10, -10, -10, -10, -10, -10, -20, //
]);
const WHITE_ROOK: HeatMap = HeatMap::new([
    0, 0, 0, 0, 0, 0, 0, 0, //
    5, 10, 10, 10, 10, 10, 10, 5, //
    -5, 0, 0, 0, 0, 0, 0, -5, //
    -5, 0, 0, 0, 0, 0, 0, -5, //
    -5, 0, 0, 0, 0, 0, 0, -5, //
    -5, 0, 0, 0, 0, 0, 0, -5, //
    -5, 0, 0, 0, 0, 0, 0, -5, //
    0, 0, 0, 5, 5, 0, 0, 0, //
]);
const WHITE_QUEEN: HeatMap = HeatMap::new([
    -20, -10, -10, -5, -5, -10, -10, -20, //
    -10, 0, 0, 0, 0, 0, 0, -10, //
    -10, 0, 5, 5, 5, 5, 0, -10, //
    -5, 0, 5, 5, 5, 5, 0, -5, //
    0, 0, 5, 5, 5, 5, 0, -5, //
    -10, 5, 5, 5, 5, 5, 0, -10, //
    -10, 0, 5, 0, 0, 0, 0, -10, //
    -20, -10, -10, -5, -5, -10, -10, -20, //
]);
const WHITE_KING_OPENING: HeatMap = HeatMap::new([
    -30, -40, -40, -50, -50, -40, -40, -30, //
    -30, -40, -40, -50, -50, -40, -40, -30, //
    -30, -40, -40, -50, -50, -40, -40, -30, //
    -30, -40, -40, -50, -50, -40, -40, -30, //
    -20, -30, -30, -40, -40, -30, -30, -20, //
    -10, -20, -20, -20, -20, -20, -20, -10, //
    20, 20, 0, 0, 0, 0, 20, 20, //
    20, 30, 10, 0, 0, 10, 30, 20, //
]);
const WHITE_KING_ENDGAME: HeatMap = HeatMap::new([
    -50, -40, -30, -20, -20, -30, -40, -50, //
    -30, -20, -10, 0, 0, -10, -20, -30, //
    -30, -10, 20, 30, 30, 20, -10, -30, //
    -30, -10, 30, 40, 40, 30, -10, -30, //
    -30, -10, 30, 40, 40, 30, -10, -30, //
    -30, -10, 20, 30, 30, 20, -10, -30, //
    -30, -30, 0, 0, 0, 0, -30, -30, //
    -50, -30, -30, -30, -30, -30, -30, -50, //
]);

fn pawn_singlet_position(board: &Board, phase: GamePhase) -> f32 {
    // TODO: Make below const
    let BLACK_PAWN: HeatMap = WHITE_PAWN.mirror_copy();
    let BLACK_KNIGHT: HeatMap = WHITE_KNIGHT.mirror_copy();
    let BLACK_BISHOP: HeatMap = WHITE_BISHOP.mirror_copy();
    let BLACK_ROOK: HeatMap = WHITE_ROOK.mirror_copy();
    let BLACK_QUEEN: HeatMap = WHITE_QUEEN.mirror_copy();
    let BLACK_KING_OPENING: HeatMap = WHITE_KING_OPENING.mirror_copy();
    let BLACK_KING_ENDGAME: HeatMap = WHITE_KING_ENDGAME.mirror_copy();
    let white_pawns = board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Pawn);
    let black_pawns = board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Pawn);
    // match phase {
    //     GamePhase::Opening => {
    //         OPENING_WHITE_PAWN_HM.dot(&white_pawns) - OPENING_BLACK_PAWN_HM.dot(&black_pawns)
    //     }
    //     GamePhase::MiddleGame => 0.0,
    //     GamePhase::Endgame => {
    //         LATE_WHITE_PAWN_HM.dot(&white_pawns) - LATE_BLACK_PAWN_HM.dot(&black_pawns)
    //     }
    // }
    0.0
}
