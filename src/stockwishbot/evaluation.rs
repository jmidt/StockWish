// Evaluation of a board state. Usually used for leaf nodes in the game tree. Positive values are good for white,
// negative values are good for black.
use chess::BitBoard;
use chess::Board;
use chess::BoardStatus;
use chess::ALL_SQUARES;

use super::cache::insert_in_cache_if_better;
use super::cache::SWCache;
use super::cache::Score;
use super::cache::TopTargets;
use super::move_ordering::moves_toward_quiescence;
use super::Calibration;

const PIECE_VALUE_SCALE: i32 = 12;
const POSITIONAL_SCALE: i32 = 1;
const QUEEN_VALUE: i32 = 900;
const ROOK_VALUE: i32 = 500;
const BISHOP_VALUE: i32 = 330;
const KNIGHT_VALUE: i32 = 320;
const PAWN_VALUE: i32 = 100;

pub fn quiescent_board_score(
    board: &Board,
    cache: &mut SWCache,
    alpha: i32,
    beta: i32,
    calibration: Calibration,
) -> i32 {
    // Evaluate a board. We only actually evaluate quiescent board states, so we run through
    // a new game tree, with no max depth, only considering captures.
    // TODO: Currently using alpha-beta pruning, but I hear delta-pruning is good at this?
    let score = quiescent_alpha_beta(board, alpha, beta, calibration);
    // TODO: We could potentially find some good targets, but it would only involve captures,
    // so probably not so useful for general tree search.
    insert_in_cache_if_better(board, 0, &score, TopTargets::new(0), cache);
    score.into()
}

// NOTE: Currently not using a cache. I think this is best, but tests should be done.
fn quiescent_alpha_beta(board: &Board, _alpha: i32, beta: i32, calibration: Calibration) -> Score {
    // Check if current raw_board_score is enough to cause a beta-cutoff
    let eval = raw_board_score(board, calibration);
    if beta <= eval {
        return Score::LowerBound(eval);
    }
    // Possibly raise alpha
    let mut alpha = std::cmp::max(_alpha, eval);
    for capture in moves_toward_quiescence(board) {
        // TODO: If current eval + captured piece (+ some margin) is above alpha, quiesce further down.
        // Otherwise set best_value = max(best_value, that-thing-above^^)
        let child_score =
            -quiescent_alpha_beta(&board.make_move_new(capture), -beta, -alpha, calibration);
        let child_score_numeric = i32::from(child_score);
        if beta <= child_score_numeric {
            return Score::LowerBound(child_score_numeric);
        }
        alpha = std::cmp::max(alpha, child_score_numeric);
    }
    // If we get here, we HAD possible captures to do, and return the score for the optimal capture.
    Score::Exact(alpha)
}

pub fn raw_board_score(board: &Board, calibration: Calibration) -> i32 {
    // This function must return scores from the point-of-view of the player who's turn it is.
    match board.status() {
        // If it is currently a checkmate, it is a very bad thing for the current player
        BoardStatus::Checkmate => i32::MIN + 1,
        // A stalemate is evenly meh.
        BoardStatus::Stalemate => 0,
        _ => ongoing_raw_board_score(board, calibration),
    }
}

fn ongoing_raw_board_score(board: &Board, calibration: Calibration) -> i32 {
    // This function must return scores from the point-of-view of the player who's turn it is.
    let material = sum_piece_square_tables(board);
    // let mobility = mobility_score(board);
    let turn = match board.side_to_move() {
        chess::Color::White => 1,
        chess::Color::Black => -1,
    };
    turn * material
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
pub fn piece_value(p: Option<chess::Piece>) -> i32 {
    match p {
        Some(chess::Piece::Queen) => 900,
        Some(chess::Piece::Rook) => 500,
        Some(chess::Piece::Bishop) => 330,
        Some(chess::Piece::Knight) => 320,
        Some(chess::Piece::Pawn) => 100,
        _ => 0, // The king is covered by the checkmate rules.
    }
}

struct PieceSquareTable {
    cells: [i32; 64], // Row-major: a1, b1, c1, ...
}

impl PieceSquareTable {
    pub const fn new(offset: i32, offset_scale: i32, scale: i32, vals: [i32; 64]) -> Self {
        let mut cells = vals;
        let mut i = 0;
        while i < 64 {
            cells[i] = offset * offset_scale + scale * cells[i];
            i += 1;
        }
        Self { cells }
    }

    pub const fn new_raw(vals: [i32; 64]) -> Self {
        Self { cells: vals }
    }

    pub fn dot(&self, bitboard: &BitBoard) -> i32 {
        bitboard
            .into_iter()
            .map(|sq| self.cells[63 - sq.to_index()])
            .sum()
    }

    pub const fn change_color(&self) -> Self {
        // The mirror is chunk-reversed, to account for the king and queen that change position.
        Self::new_raw([
            self.cells[56],
            self.cells[57],
            self.cells[58],
            self.cells[59],
            self.cells[60],
            self.cells[61],
            self.cells[62],
            self.cells[63],
            self.cells[48],
            self.cells[49],
            self.cells[50],
            self.cells[51],
            self.cells[52],
            self.cells[53],
            self.cells[54],
            self.cells[55],
            self.cells[40],
            self.cells[41],
            self.cells[42],
            self.cells[43],
            self.cells[44],
            self.cells[45],
            self.cells[46],
            self.cells[47],
            self.cells[32],
            self.cells[33],
            self.cells[34],
            self.cells[35],
            self.cells[36],
            self.cells[37],
            self.cells[38],
            self.cells[39],
            self.cells[24],
            self.cells[25],
            self.cells[26],
            self.cells[27],
            self.cells[28],
            self.cells[29],
            self.cells[30],
            self.cells[31],
            self.cells[16],
            self.cells[17],
            self.cells[18],
            self.cells[19],
            self.cells[20],
            self.cells[21],
            self.cells[22],
            self.cells[23],
            self.cells[8],
            self.cells[9],
            self.cells[10],
            self.cells[11],
            self.cells[12],
            self.cells[13],
            self.cells[14],
            self.cells[15],
            self.cells[0],
            self.cells[57],
            self.cells[58],
            self.cells[59],
            self.cells[60],
            self.cells[61],
            self.cells[62],
            self.cells[57],
        ])
    }
}

fn sum_piece_square_tables(board: &Board) -> i32 {
    let (white_pawns, white_knights, white_bishops, white_rooks, white_queens, white_king) =
        piece_square_tables_for_color(board, chess::Color::White);
    let (black_pawns, black_knights, black_bishops, black_rooks, black_queens, black_king) =
        piece_square_tables_for_color(board, chess::Color::Black);
    WHITE_PAWN.dot(&white_pawns) - BLACK_PAWN.dot(&black_pawns) + WHITE_KNIGHT.dot(&white_knights)
        - BLACK_KNIGHT.dot(&black_knights)
        + WHITE_BISHOP.dot(&white_bishops)
        - BLACK_BISHOP.dot(&black_bishops)
        + WHITE_ROOK.dot(&white_rooks)
        - BLACK_ROOK.dot(&black_rooks)
        + WHITE_QUEEN.dot(&white_queens)
        - BLACK_QUEEN.dot(&black_queens)
        + WHITE_KING_OPENING.dot(&white_king)
        - BLACK_KING_OPENING.dot(&black_king)
}

fn piece_square_tables_for_color(
    board: &Board,
    color: chess::Color,
) -> (
    chess::BitBoard,
    chess::BitBoard,
    chess::BitBoard,
    chess::BitBoard,
    chess::BitBoard,
    chess::BitBoard,
) {
    let pawns = board.color_combined(color) & board.pieces(chess::Piece::Pawn);
    let knights = board.color_combined(color) & board.pieces(chess::Piece::Knight);
    let bishops = board.color_combined(color) & board.pieces(chess::Piece::Bishop);
    let rooks = board.color_combined(color) & board.pieces(chess::Piece::Rook);
    let queens = board.color_combined(color) & board.pieces(chess::Piece::Queen);
    let king = board.color_combined(color) & board.pieces(chess::Piece::King);
    (pawns, knights, bishops, rooks, queens, king)
}

// Pawns
const WHITE_PAWN: PieceSquareTable = PieceSquareTable::new(
    PAWN_VALUE,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        0, 0, 0, 0, 0, 0, 0, 0, //
        50, 50, 50, 50, 50, 50, 50, 50, //
        10, 10, 20, 30, 30, 20, 10, 10, //
        5, 5, 10, 25, 25, 10, 5, 5, //
        0, 0, 0, 20, 20, 0, 0, 0, //
        5, -5, -10, 0, 0, -10, -5, 5, //
        5, 10, 10, -20, -20, 10, 10, 5, //
        0, 0, 0, 0, 0, 0, 0, 0, //
    ],
);
const WHITE_KNIGHT: PieceSquareTable = PieceSquareTable::new(
    KNIGHT_VALUE,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        -50, -40, -30, -30, -30, -30, -40, -50, //
        -40, -20, 0, 0, 0, 0, -20, -40, //
        -30, 0, 10, 15, 15, 10, 0, -30, //
        -30, 5, 15, 20, 20, 15, 5, -30, //
        -30, 0, 15, 20, 20, 15, 0, -30, //
        -30, 5, 10, 15, 15, 10, 5, -30, //
        -40, -20, 0, 5, 5, 0, -20, -40, //
        -50, -40, -30, -30, -30, -30, -40, -50, //
    ],
);
const WHITE_BISHOP: PieceSquareTable = PieceSquareTable::new(
    BISHOP_VALUE,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        -20, -10, -10, -10, -10, -10, -10, -20, //
        -10, 0, 0, 0, 0, 0, 0, -10, //
        -10, 0, 5, 10, 10, 5, 0, -10, //
        -10, 5, 5, 10, 10, 5, 5, -10, //
        -10, 0, 10, 10, 10, 10, 0, -10, //
        -10, 10, 10, 10, 10, 10, 10, -10, //
        -10, 5, 0, 0, 0, 0, 5, -10, //
        -20, -10, -10, -10, -10, -10, -10, -20, //
    ],
);
const WHITE_ROOK: PieceSquareTable = PieceSquareTable::new(
    ROOK_VALUE,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        0, 0, 0, 0, 0, 0, 0, 0, //
        5, 10, 10, 10, 10, 10, 10, 5, //
        -5, 0, 0, 0, 0, 0, 0, -5, //
        -5, 0, 0, 0, 0, 0, 0, -5, //
        -5, 0, 0, 0, 0, 0, 0, -5, //
        -5, 0, 0, 0, 0, 0, 0, -5, //
        -5, 0, 0, 0, 0, 0, 0, -5, //
        0, 0, 0, 5, 5, 0, 0, 0, //
    ],
);
const WHITE_QUEEN: PieceSquareTable = PieceSquareTable::new(
    QUEEN_VALUE,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        -20, -10, -10, -5, -5, -10, -10, -20, //
        -10, 0, 0, 0, 0, 0, 0, -10, //
        -10, 0, 5, 5, 5, 5, 0, -10, //
        -5, 0, 5, 5, 5, 5, 0, -5, //
        0, 0, 5, 5, 5, 5, 0, -5, //
        -10, 5, 5, 5, 5, 5, 0, -10, //
        -10, 0, 5, 0, 0, 0, 0, -10, //
        -20, -10, -10, -5, -5, -10, -10, -20, //
    ],
);
const WHITE_KING_OPENING: PieceSquareTable = PieceSquareTable::new(
    0,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        -30, -40, -40, -50, -50, -40, -40, -30, //
        -30, -40, -40, -50, -50, -40, -40, -30, //
        -30, -40, -40, -50, -50, -40, -40, -30, //
        -30, -40, -40, -50, -50, -40, -40, -30, //
        -20, -30, -30, -40, -40, -30, -30, -20, //
        -10, -20, -20, -20, -20, -20, -20, -10, //
        20, 20, 0, 0, 0, 0, 20, 20, //
        20, 30, 10, 0, 0, 10, 30, 20, //
    ],
);
const WHITE_KING_ENDGAME: PieceSquareTable = PieceSquareTable::new(
    0,
    PIECE_VALUE_SCALE,
    POSITIONAL_SCALE,
    [
        -50, -40, -30, -20, -20, -30, -40, -50, //
        -30, -20, -10, 0, 0, -10, -20, -30, //
        -30, -10, 20, 30, 30, 20, -10, -30, //
        -30, -10, 30, 40, 40, 30, -10, -30, //
        -30, -10, 30, 40, 40, 30, -10, -30, //
        -30, -10, 20, 30, 30, 20, -10, -30, //
        -30, -30, 0, 0, 0, 0, -30, -30, //
        -50, -30, -30, -30, -30, -30, -30, -50, //
    ],
);

const BLACK_PAWN: PieceSquareTable = WHITE_PAWN.change_color();
const BLACK_KNIGHT: PieceSquareTable = WHITE_KNIGHT.change_color();
const BLACK_BISHOP: PieceSquareTable = WHITE_BISHOP.change_color();
const BLACK_ROOK: PieceSquareTable = WHITE_ROOK.change_color();
const BLACK_QUEEN: PieceSquareTable = WHITE_QUEEN.change_color();
const BLACK_KING_OPENING: PieceSquareTable = WHITE_KING_OPENING.change_color();
const BLACK_KING_ENDGAME: PieceSquareTable = WHITE_KING_ENDGAME.change_color();

// fn pawn_singlet_position(board: &Board, phase: GamePhase) -> f32 {
//     // TODO: Make below const
//     let BLACK_PAWN: PieceSquareTable = WHITE_PAWN.mirror_copy();
//     let BLACK_KNIGHT: PieceSquareTable = WHITE_KNIGHT.mirror_copy();
//     let BLACK_BISHOP: PieceSquareTable = WHITE_BISHOP.mirror_copy();
//     let BLACK_ROOK: PieceSquareTable = WHITE_ROOK.mirror_copy();
//     let BLACK_QUEEN: PieceSquareTable = WHITE_QUEEN.mirror_copy();
//     let BLACK_KING_OPENING: PieceSquareTable = WHITE_KING_OPENING.mirror_copy();
//     let BLACK_KING_ENDGAME: PieceSquareTable = WHITE_KING_ENDGAME.mirror_copy();
//     let white_pawns = board.color_combined(chess::Color::White) & board.pieces(chess::Piece::Pawn);
//     let black_pawns = board.color_combined(chess::Color::Black) & board.pieces(chess::Piece::Pawn);
//     // match phase {
//     //     GamePhase::Opening => {
//     //         OPENING_WHITE_PAWN_HM.dot(&white_pawns) - OPENING_BLACK_PAWN_HM.dot(&black_pawns)
//     //     }
//     //     GamePhase::MiddleGame => 0.0,
//     //     GamePhase::Endgame => {
//     //         LATE_WHITE_PAWN_HM.dot(&white_pawns) - LATE_BLACK_PAWN_HM.dot(&black_pawns)
//     //     }
//     // }
//     0.0
// }
