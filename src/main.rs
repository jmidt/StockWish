use chess::Board;
use chess::ChessMove;
use chess::Color;
use chess::MoveGen;
use chess::EMPTY;

use chess::Piece;
use chess::Rank;
use chess::Square;
use eframe::egui;
use egui::epaint::RectShape;
use egui::pos2;
use egui::Color32;
use egui::Context;
use egui::Frame;
use egui::Pos2;
use egui::Rect;
use egui::Rounding;
use egui::Sense;
use egui::Shape;
use egui::Style;
use egui::Ui;
use egui::Vec2;
use stockwish::StockWish;

mod stockwish;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 800.0)),
        ..Default::default()
    };
    eframe::run_native("ChessBot", options, Box::new(|_cc| Box::<MyApp>::default()))
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum PromotionChoice {
    NotNeeded,
    Pending,
    Piece(Piece),
}

struct MyApp {
    // The all-important board state
    board: Board,
    // UI
    board_image: egui_extras::RetainedImage,
    king_black: egui_extras::RetainedImage,
    king_white: egui_extras::RetainedImage,
    queen_black: egui_extras::RetainedImage,
    queen_white: egui_extras::RetainedImage,
    rook_black: egui_extras::RetainedImage,
    rook_white: egui_extras::RetainedImage,
    bishop_black: egui_extras::RetainedImage,
    bishop_white: egui_extras::RetainedImage,
    knight_black: egui_extras::RetainedImage,
    knight_white: egui_extras::RetainedImage,
    pawn_black: egui_extras::RetainedImage,
    pawn_white: egui_extras::RetainedImage,
    // Dialogs, etc.
    promotion_choice: PromotionChoice,
    // The currently chosen piece is on this square. This is ready to move
    chosen_piece: Option<chess::Square>,
    // Only used to store information while the user is choosing a promotion
    chosen_dest_square: Option<chess::Square>,
    // The all-important chess AI
    chess_ai: stockwish::StockWish,
    ai_color: Color,
}

impl MyApp {
    fn fetch_piece_image(&self, piece: Piece, color: Color) -> &egui_extras::RetainedImage {
        if color == Color::White {
            match piece {
                Piece::King => &self.king_white,
                Piece::Queen => &self.queen_white,
                Piece::Rook => &self.rook_white,
                Piece::Bishop => &self.bishop_white,
                Piece::Knight => &self.knight_white,
                Piece::Pawn => &self.pawn_white,
            }
        } else {
            match piece {
                Piece::King => &self.king_black,
                Piece::Queen => &self.queen_black,
                Piece::Rook => &self.rook_black,
                Piece::Bishop => &self.bishop_black,
                Piece::Knight => &self.knight_black,
                Piece::Pawn => &self.pawn_black,
            }
        }
    }

    fn click_square(&mut self, square: Square, promotion: PromotionChoice) {
        println!("Clicking {} with {:?}", square, promotion);
        if let Some(chosen_piece) = self.chosen_piece {
            // Clicked a square with a chosen piece. This is an attempted move
            self.attempt_move(chosen_piece, square, promotion);
            self.chosen_piece = None;
        } else {
            // Clicked a square with no piece chosen. This chooses the piece
            self.chosen_piece = Some(square);
        }
        // If it is the AI's turn, take an action
        if self.board.side_to_move() == self.ai_color {
            self.let_ai_move();
        }
    }

    // TODO: Allow promotion
    fn attempt_move(
        &mut self,
        src_square: Square,
        dest_square: Square,
        promotion: PromotionChoice,
    ) {
        let chess_move = match promotion {
            PromotionChoice::Piece(p) => chess::ChessMove::new(src_square, dest_square, Some(p)),
            _ => chess::ChessMove::new(src_square, dest_square, None),
        };
        self.take_move(chess_move);
    }

    fn let_ai_move(&mut self) {
        if let Some(ai_move) = self.chess_ai.best_next_move(&self.board) {
            self.take_move(ai_move);
        } else {
            println!("AI is checkmate! You win!")
        }
    }

    fn take_move(&mut self, chess_move: ChessMove) {
        if self.board.legal(chess_move) {
            let mut tmp_board = self.board;
            self.board.make_move(chess_move, &mut tmp_board);
            self.board = tmp_board;
        }
    }
}

macro_rules! svg_image {
    ($name:literal, $fit_to:expr) => {
        egui_extras::RetainedImage::from_svg_bytes_with_size(
            $name,
            include_bytes!(concat!("../assets/", $name, ".svg")),
            $fit_to,
        )
        .unwrap()
    };
}

macro_rules! svg_image_board {
    ($name:literal) => {
        svg_image!($name, egui_extras::image::FitTo::Original)
    };
}

macro_rules! svg_image_piece {
    ($name:literal) => {
        svg_image!($name, egui_extras::image::FitTo::Size(80, 80))
    };
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            board: Board::default(),
            board_image: svg_image_board!("chessboard"),
            king_black: svg_image_piece!("king_black"),
            king_white: svg_image_piece!("king_white"),
            queen_black: svg_image_piece!("queen_black"),
            queen_white: svg_image_piece!("queen_white"),
            rook_black: svg_image_piece!("rook_black"),
            rook_white: svg_image_piece!("rook_white"),
            bishop_black: svg_image_piece!("bishop_black"),
            bishop_white: svg_image_piece!("bishop_white"),
            knight_black: svg_image_piece!("knight_black"),
            knight_white: svg_image_piece!("knight_white"),
            pawn_black: svg_image_piece!("pawn_black"),
            pawn_white: svg_image_piece!("pawn_white"),
            promotion_choice: PromotionChoice::NotNeeded,
            chosen_piece: None,
            chosen_dest_square: None,
            chess_ai: StockWish::default(),
            ai_color: Color::Black,
        }
    }
}

fn square_to_pos(square: Square, board_size: Vec2) -> Pos2 {
    let x = ((square.get_file().to_index() as f32) + 0.5) * board_size.x / 8.0;
    let y = (7.0 - (square.get_rank().to_index() as f32) + 0.5) * board_size.y / 8.0;
    pos2(x, y)
}

fn square_to_rect(square: Square, board_size: Vec2) -> Rect {
    let rect_size = Vec2::new(board_size.x / 8.0, board_size.y / 8.0);
    egui::Rect::from_center_size(square_to_pos(square, board_size), rect_size)
}

fn pos_to_square(pos: Pos2, board_size: Vec2) -> Square {
    let rank_index = 7 - (pos.y * 8.0 / board_size.y) as usize;
    let file_index = (pos.x * 8.0 / board_size.x) as usize;
    Square::make_square(
        chess::Rank::from_index(rank_index),
        chess::File::from_index(file_index),
    )
}

// Check if this move would require a promotion
fn attempting_promotion(board: &Board, src_square: Square, dest_square: Square) -> bool {
    board.piece_on(src_square) == Some(chess::Piece::Pawn)
        && (src_square.get_rank() == Rank::Second && dest_square.get_rank() == Rank::First
            || src_square.get_rank() == Rank::Seventh && dest_square.get_rank() == Rank::Eighth)
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut square_clicked: Option<Square> = None;

        egui::Area::new("pieces")
            .default_pos(egui::pos2(0.0, 0.0))
            .movable(false)
            .enabled(true)
            .interactable(false)
            .show(ctx, |ui| {
                // Centers
                let board_size = ui.available_size();
                let piece_size = board_size / 10.0;

                // Paint chosen piece marker
                if let Some(chosen_piece) = self.chosen_piece {
                    let shape = Shape::rect_filled(
                        square_to_rect(chosen_piece, board_size),
                        Rounding::none(),
                        Color32::LIGHT_GREEN,
                    );
                    ui.painter().add(shape);
                }

                // Paint pieces on the board
                for piece in chess::ALL_PIECES {
                    for color in chess::ALL_COLORS {
                        let pieces = self.board.color_combined(color) & self.board.pieces(piece);
                        for square in pieces {
                            let piece_image = egui::Image::new(
                                self.fetch_piece_image(piece, color).texture_id(ctx),
                                piece_size,
                            );
                            piece_image.paint_at(ui, square_to_rect(square, board_size));
                        }
                    }
                }

                // Paint possible moves
                if let Some(chosen_piece) = self.chosen_piece {
                    for legal_move in MoveGen::new_legal(&self.board) {
                        if legal_move.get_source() == chosen_piece {
                            let shape = Shape::circle_filled(
                                square_to_pos(legal_move.get_dest(), board_size),
                                piece_size.x / 6.0,
                                Color32::GRAY.gamma_multiply(0.5),
                            );
                            ui.painter().add(shape);
                        }
                    }
                }

                // Possible promotion dialog
                if self.promotion_choice == PromotionChoice::Pending {
                    egui::Window::new("Promotion").show(ctx, |ui| {
                        ui.set_min_width(400.0); // if you want to control the size
                        if ui.button("Queen").clicked() {
                            self.promotion_choice = PromotionChoice::Piece(Piece::Queen);
                            square_clicked = self.chosen_dest_square;
                        }
                        if ui.button("Rook").clicked() {
                            self.promotion_choice = PromotionChoice::Piece(Piece::Rook);
                            square_clicked = self.chosen_dest_square;
                        }
                        if ui.button("Bishop").clicked() {
                            self.promotion_choice = PromotionChoice::Piece(Piece::Bishop);
                            square_clicked = self.chosen_dest_square;
                        }
                        if ui.button("Knight").clicked() {
                            self.promotion_choice = PromotionChoice::Piece(Piece::Knight);
                            square_clicked = self.chosen_dest_square;
                        }
                    });
                }
            });

        // Show board and handle clicks
        let central_panel_frame = Frame::default().outer_margin(0.0);
        egui::CentralPanel::default()
            .frame(central_panel_frame)
            .show(ctx, |ui| {
                // If we are currently choosing a promotion, do not let the user click
                ui.set_enabled(self.promotion_choice != PromotionChoice::Pending);

                let board_size = ui.available_size();
                let board_response = ui.add(
                    egui::Image::new(self.board_image.texture_id(ctx), board_size)
                        .sense(Sense::click()),
                );
                if board_response.clicked() {
                    let click_position = board_response.interact_pointer_pos();
                    square_clicked = Some(pos_to_square(click_position.unwrap(), board_size));
                }
            });

        // Handle mouse clicks
        if let Some(dest_sq) = square_clicked {
            // Choose Promotion if a pawn is reaching the end
            if self.chosen_piece.is_some()
                && attempting_promotion(&self.board, self.chosen_piece.unwrap(), dest_sq)
                && self.promotion_choice == PromotionChoice::NotNeeded
            {
                println!("HERE");
                self.chosen_dest_square = Some(dest_sq);
                self.promotion_choice = PromotionChoice::Pending;
            } else {
                self.click_square(dest_sq, self.promotion_choice);
                self.promotion_choice = PromotionChoice::NotNeeded;
            }
        }
    }
}
