use chess::{BitBoard, Board, ChessMove, MoveGen};

use super::cache::TopTargets;
use super::evaluation::piece_value;

//
// A better move order for iteration, hitting potentially high-value moves earlier
//
enum MoveOrderStage {
    Favored,
    Remaining,
}

// TODO: Can be simplified, to just have an implied stage.
pub struct MoveOrder {
    favored: Vec<ChessMove>,
    remaining_movegen: MoveGen,
    board: Board,
    stage: MoveOrderStage,
}

impl MoveOrder {
    fn movegen_from_mask(board: &Board, mask: BitBoard) -> MoveGen {
        let mut movegen = MoveGen::new_legal(board);
        movegen.set_iterator_mask(mask);
        movegen
    }

    fn mvv_lva(board: &Board, chess_move: &ChessMove) -> i32 {
        // The tentative score of a capture, as value of victim minus value of attacker
        let victim = board.piece_on(chess_move.get_dest());
        let attacker = board.piece_on(chess_move.get_source());
        piece_value(victim) - piece_value(attacker)
    }

    fn ordered_captures(board: &Board, blacklist: Option<&BitBoard>) -> Vec<ChessMove> {
        // All captures in MVV-LVA ordering
        let other_players_pieces = board.color_combined(!board.side_to_move());
        let mut captures: Vec<ChessMove> = if let Some(b) = blacklist {
            Self::movegen_from_mask(board, *other_players_pieces & !b).collect()
        } else {
            Self::movegen_from_mask(board, *other_players_pieces).collect()
        };
        captures.sort_by(|a, b| (Self::mvv_lva(board, a)).cmp(&Self::mvv_lva(board, b)));
        captures
    }

    pub fn new_with_hint(board: &Board, top_targets: TopTargets) -> Self {
        // Construct a MoveOrder in the `Hints` stage.
        let targets_bitboard = top_targets.targets();
        let other_players_pieces = board.color_combined(!board.side_to_move());
        let mut favored = Self::ordered_captures(board, Some(&targets_bitboard));
        let remaining_movegen =
            Self::movegen_from_mask(board, !other_players_pieces & !targets_bitboard);
        // Collected list
        favored.extend(top_targets.ordered_moves());
        Self {
            favored,
            remaining_movegen,
            board: *board,
            stage: MoveOrderStage::Favored,
        }
    }

    pub fn new(board: &Board) -> Self {
        // Construct a MoveOrder in the `Captures` stage.
        let other_players_pieces = board.color_combined(!board.side_to_move());
        let favored = Self::ordered_captures(board, None);
        let remaining_movegen = Self::movegen_from_mask(board, !other_players_pieces);
        Self {
            favored,
            remaining_movegen,
            board: *board,
            stage: MoveOrderStage::Favored,
        }
    }
}

impl ExactSizeIterator for MoveOrder {
    /// Give the exact length of this iterator
    fn len(&self) -> usize {
        self.favored.len() + self.remaining_movegen.len()
    }
}

impl Iterator for MoveOrder {
    type Item = ChessMove;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stage {
            MoveOrderStage::Favored => {
                if let Some(next) = self.favored.pop() {
                    // In the Favored phase, we take from a pregenerated vector of potentially good moves.
                    Some(next)
                } else {
                    // When empty, move on the the Remaining phase
                    self.stage = MoveOrderStage::Remaining;
                    self.next()
                }
            }
            MoveOrderStage::Remaining => self.remaining_movegen.next(),
        }
    }
}
