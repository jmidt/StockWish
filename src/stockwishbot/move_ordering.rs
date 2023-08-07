use chess::{BitBoard, Board, ChessMove, MoveGen, EMPTY};
use itertools::Itertools;

use super::cache::TopTargets;
use super::evaluation::piece_value;
//
// A better move order for iteration, hitting potentially high-value moves earlier
//

// Note that intended behaviour is:
// First criterion: Enum variant
// Second Criterion: Inner value
#[derive(Eq, PartialEq, PartialOrd, Ord)]
enum MoveCategory {
    NormalMove(i32),
    Capture(i32),
    Promotion(i32),
    Cached(i32),
}

fn mvv_lva(board: &Board, chess_move: &ChessMove) -> i32 {
    // The tentative score of a capture, as value of victim minus value of attacker
    let victim = board.piece_on(chess_move.get_dest());
    let attacker = board.piece_on(chess_move.get_source());
    piece_value(victim) - piece_value(attacker)
}

fn move_score(
    a: &ChessMove,
    board: &Board,
    other_players_pieces: &BitBoard,
    cache_moves_opt: &Option<Vec<ChessMove>>,
) -> MoveCategory {
    // Moves in the cache get top priority
    if let Some(cache_moves) = cache_moves_opt {
        if let Some(pos) = cache_moves.iter().position(|x| x == a) {
            return MoveCategory::Cached(pos.try_into().unwrap());
        }
    }
    // Promotions are next in line
    if let Some(promotion_piece) = a.get_promotion() {
        return MoveCategory::Promotion(piece_value(Some(promotion_piece)));
    }
    // Captures are ranked after MVV-LVA
    if other_players_pieces & BitBoard::from_square(a.get_dest()) != BitBoard::new(0) {
        return MoveCategory::Capture(mvv_lva(board, a));
    }
    // Non-captures, non-promotions are then considered equally boring
    MoveCategory::NormalMove(0)
}

pub fn generate_move_order(board: &Board, top_targets: Option<TopTargets>) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    let other_players_pieces = board.color_combined(!board.side_to_move());
    let cache_moves_opt = top_targets.map(|t| t.ordered_moves());
    // Now we sort in descending order, putting the good stuff first
    moves.sort_by_key(|a| {
        std::cmp::Reverse(move_score(a, board, other_players_pieces, &cache_moves_opt))
    });
    moves
}

pub fn moves_toward_quiescence(board: &Board) -> Vec<ChessMove> {
    if *board.checkers() != EMPTY {
        // We are in check. In this case we consider all possible moves
        return MoveGen::new_legal(board).collect_vec();
    }
    // Otherwise, we return all captures
    let mut movegen = MoveGen::new_legal(board);
    let other_players_pieces = board.color_combined(!board.side_to_move());
    movegen.set_iterator_mask(*other_players_pieces);
    let mut moves: Vec<ChessMove> = movegen.collect_vec();
    // Sort in descending order, putting the good stuff first
    moves.sort_by_key(|a| std::cmp::Reverse(mvv_lva(board, a)));
    moves
}
