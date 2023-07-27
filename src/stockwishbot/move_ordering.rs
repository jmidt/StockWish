use chess::{BitBoard, Board, ChessMove, MoveGen};

use super::cache::TopTargets;

//
// A better move order for iteration, hitting potentially high-value moves earlier
//
enum MoveOrderStage {
    Hints,
    Captures,
    Remaining,
}

// TODO: Can be simplified, to just have an implied stage.
pub struct MoveOrder {
    targets: Option<Vec<ChessMove>>, // Should be reversed with first at the end, so we can pop from it
    captures_movegen: MoveGen,
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

    fn movegen_from_ignore_mask(board: &Board, mask: BitBoard) -> MoveGen {
        let mut movegen = MoveGen::new_legal(board);
        movegen.remove_mask(mask);
        movegen
    }

    pub fn new_with_hint(board: &Board, top_targets: TopTargets) -> Self {
        // Construct a MoveOrder in the `Hints` stage.
        let targets_bitboard = top_targets.targets();
        let other_players_pieces = board.color_combined(!board.side_to_move());
        let captures_movegen =
            Self::movegen_from_mask(board, *other_players_pieces & !targets_bitboard);
        let remaining_movegen =
            Self::movegen_from_mask(board, !other_players_pieces & !targets_bitboard);
        Self {
            targets: Some(top_targets.ordered_moves()),
            captures_movegen,
            remaining_movegen,
            board: *board,
            stage: MoveOrderStage::Hints,
        }
    }

    pub fn new(board: &Board) -> Self {
        // Construct a MoveOrder in the `Captures` stage.
        let other_players_pieces = board.color_combined(!board.side_to_move());
        let captures_movegen = Self::movegen_from_mask(board, *other_players_pieces);
        let remaining_movegen = Self::movegen_from_mask(board, !other_players_pieces);
        Self {
            targets: None,
            captures_movegen,
            remaining_movegen,
            board: *board,
            stage: MoveOrderStage::Captures,
        }
    }
}

impl ExactSizeIterator for MoveOrder {
    /// Give the exact length of this iterator
    fn len(&self) -> usize {
        self.captures_movegen.len()
            + self.remaining_movegen.len()
            + if let Some(v) = &self.targets {
                v.len()
            } else {
                0
            }
    }
}

impl Iterator for MoveOrder {
    type Item = ChessMove;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stage {
            MoveOrderStage::Hints => {
                if let Some(ref mut targets) = self.targets {
                    if let Some(next) = targets.pop() {
                        // In the hints-phase, we take from the hinted_moves member
                        Some(next)
                    } else {
                        // When empty, move to the Captures phase
                        self.stage = MoveOrderStage::Captures;
                        self.next()
                    }
                } else {
                    unreachable!("No list of hints for a hinted moveorder!")
                }
            }
            MoveOrderStage::Captures => {
                if let Some(next) = self.captures_movegen.next() {
                    // In the captures phase, we iterate from a MoveGen that should be set to only use captures
                    // When coming from the hints-phase, we should not repeat already checked moves.
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
