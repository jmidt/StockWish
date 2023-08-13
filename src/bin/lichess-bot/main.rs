#![allow(unused_imports)]
#![allow(unused_macros)]

use chess::{BoardStatus, ChessMove, Color};
use chrono::{TimeZone, Utc};
use futures_util::TryStreamExt;
use licoricedev::client::{Lichess, LichessResult};
use licoricedev::models::board::{BoardState, GameState};
use licoricedev::models::board::{Challengee, Event};
use licoricedev::models::game::Player;
use licoricedev::models::user::{LightUser, PerfType};
use serde_json::to_string_pretty;
use std::time::Duration;
use std::{env, thread, time};

use stockwish::stockwishbot::StockWish;

#[tokio::main]
async fn main() -> LichessResult<()> {
    // let lichess = Lichess::default();
    let lichess = Lichess::new(env::var("LICHESS_PAT_0").unwrap());
    let mut stream = lichess.stream_incoming_events().await.unwrap();

    while let Some(event) = stream.try_next().await? {
        match event {
            Event::GameStart { game } => {
                println!("A new game!");
                tokio::spawn(play_game(game.gameId.clone()));
            }
            Event::GameFinish { game } => {
                println!("Winner was {}!", game.winner);
            }
            Event::Challenge { challenge } => {
                // Accept all challenges
                let _ = lichess.challenge_accept(&challenge.id).await;
            }
            _ => {
                println!("Unknown event: {:?}", event);
            }
        }
    }
    Ok(())
}

async fn play_game(id: String) {
    let lichess = Lichess::new(env::var("LICHESS_PAT_0").unwrap());
    let mut stream = lichess.stream_bot_game_state(&id).await.unwrap();
    let mut myself: Option<chess::Color> = None;
    loop {
        let bs_result = stream.try_next().await;
        if let Ok(Some(board_state)) = bs_result {
            match board_state {
                BoardState::GameFull(game_full) => {
                    if let Challengee::LightUser(white) = game_full.white {
                        println!("White username is {}", white.username);
                        if white.username == "stockwishbot" {
                            myself = Some(Color::White);
                        }
                    }
                    if let Challengee::LightUser(black) = game_full.black {
                        println!("Black username is {}", black.username);
                        if black.username == "stockwishbot" {
                            myself = Some(Color::Black);
                        }
                    }
                    if myself.is_none() {
                        panic!("Cannot figure out who I am?!");
                    }
                    if let Some(winner) = &game_full.state.winner {
                        println!("Game over. Winner is {}", winner);
                        break;
                    }
                    make_bot_move_if_own_turn(myself, game_full.state, &lichess, &id).await;
                }
                BoardState::GameState(game_state) => {
                    if let Some(winner) = &game_state.winner {
                        println!("Game over. Winner is {}", winner);
                        break;
                    }
                    make_bot_move_if_own_turn(myself, game_state, &lichess, &id).await;
                }
                _ => {}
            }
        }
    }
}

fn chess_game_from_lichess_state(game_state: GameState) -> chess::Game {
    let mut game = chess::Game::new();
    for move_text in game_state.moves.split_ascii_whitespace() {
        game.make_move(move_text.parse::<ChessMove>().unwrap());
    }
    game
}

async fn make_bot_move_if_own_turn(
    myself: Option<chess::Color>,
    game_state: GameState,
    lichess: &Lichess,
    id: &str,
) {
    const MINIMUM_MOVE_TIME: Duration = Duration::from_millis(500);
    if let Some(side) = myself {
        let game = chess_game_from_lichess_state(game_state);
        if side == game.side_to_move() {
            let mut stockwish = StockWish::default();
            let start = time::Instant::now();
            let bot_move = stockwish.best_next_move_iterative_deepening(game);
            tokio::time::sleep_until((start + MINIMUM_MOVE_TIME).into()).await;
            let _ = lichess
                .make_a_bot_move(id, &bot_move.unwrap().to_string(), false)
                .await;
        }
    }
}
