use rand::seq::IteratorRandom;

use chess::Board;
use chess::ChessMove;
use chess::MoveGen;

#[derive(Default)]
pub struct StockWish {}

impl StockWish {
    // Returns the best next move. A return-value of None means the current player is checkmate.
    pub fn best_next_move(&self, board: &Board) -> Option<ChessMove> {
        let legal_moves = MoveGen::new_legal(board);
        let mut rng = rand::thread_rng();
        return legal_moves.choose(&mut rng);
    }
}
