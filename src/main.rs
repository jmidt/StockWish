use chess::Board;
use chess::Color;
use chess::MoveGen;
use chess::EMPTY;

use chess::Piece;
use chess::Square;
use eframe::egui;
use egui::pos2;
use egui::Context;
use egui::Pos2;
use egui::Rect;
use egui::Ui;
use egui::Vec2;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 800.0)),
        ..Default::default()
    };
    eframe::run_native("ChessBot", options, Box::new(|_cc| Box::<MyApp>::default()))

    // // create an iterable
    // let mut iterable = MoveGen::new_legal(&board);

    // // make sure .len() works.
    // assert_eq!(iterable.len(), 20); // the .len() function does *not* consume the iterator

    // let mut count = 0;

    // // now, iterate over the rest of the moves
    // iterable.set_iterator_mask(!EMPTY);
    // for _ in &mut iterable {
    //     count += 1;
    //     // This move does not capture anything
    // }

    // // make sure it works
    // assert_eq!(count, 20);
}

struct MyApp {
    board: Board,
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
}

macro_rules! svg_image {
    ($name:literal) => {
        egui_extras::RetainedImage::from_svg_bytes_with_size(
            $name,
            include_bytes!(concat!("../assets/", $name, ".svg")),
            egui_extras::image::FitTo::Original,
        )
        .unwrap()
    };
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            board: Board::default(),
            board_image: svg_image!("chessboard"),
            king_black: svg_image!("king_black"),
            king_white: svg_image!("king_white"),
            queen_black: svg_image!("queen_black"),
            queen_white: svg_image!("queen_white"),
            rook_black: svg_image!("rook_black"),
            rook_white: svg_image!("rook_white"),
            bishop_black: svg_image!("bishop_black"),
            bishop_white: svg_image!("bishop_white"),
            knight_black: svg_image!("knight_black"),
            knight_white: svg_image!("knight_white"),
            pawn_black: svg_image!("pawn_black"),
            pawn_white: svg_image!("pawn_white"),
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::Area::new("pieces")
            .default_pos(egui::pos2(0.0, 0.0))
            .movable(false)
            .show(ctx, |ui| {
                // Centers
                let width = ui.max_rect().width() / 8.0;
                let board_size = ui.available_size();
                let piece_size = board_size / 10.0;

                for piece in chess::ALL_PIECES {
                    for color in chess::ALL_COLORS {
                        let pieces = self.board.color_combined(color) & self.board.pieces(piece);
                        for square in pieces {
                            let king = egui::Image::new(
                                self.fetch_piece_image(piece, color).texture_id(ctx),
                                piece_size,
                            );
                            king.paint_at(ui, square_to_rect(square, board_size));
                        }
                    }
                }
            });

        // Show board
        egui::CentralPanel::default().show(ctx, |ui| {
            let board_size = ui.available_size();
            self.board_image.show_size(ui, board_size);
        });
    }
}
