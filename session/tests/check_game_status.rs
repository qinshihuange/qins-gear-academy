#![allow(warnings)]

mod utils;

use gstd::MessageId;
use gtest::{Log, Program};
use session_io::{Action, Event, GameOverStatus, GameStatus, State};
use utils::*;

const DELAY_CHECK_STATUS_DURATION: u32 = 200;

#[test]
fn check_game_status_should_fail_when_called_by_other_actor() {
    let system = init_system();
    let proxy_program = init_programs(&system).proxy_program;

    // When: A user send action to check game status
    let user_id = 999u64;
    system.mint_to(user_id, 100000000000000000000);
    let result = proxy_program.send(
        user_id,
        Action::CheckGameStatus {
            user: user_id.into(),
            init_id: MessageId::zero(),
        },
    );
    system.run_next_block();
    // Then: Program reverts with appropriate error message
    let log = Log::builder()
        .source(PROXY_PROGRAM)
        .dest(user_id)
        .payload_bytes(final_panic_message("Callable by current program only"));
    println!("log: {:?}", log);
}

#[test]
fn check_game_status_ignore_when_completed() {
    let system = init_system();
    let proxy_program = init_programs(&system).proxy_program;

    // Given: maximum number of attempts is reached
    proxy_program.send(USER, Action::StartGame);
    system.run_next_block();
    for _ in 0..5 {
        proxy_program.send(
            USER,
            Action::CheckWord {
                word: WRONG_ANSWER.into(),
            },
        );
        system.run_next_block();
    }
    system.run_next_block();
    let State { players, .. } = proxy_program.read_state(0).unwrap();
    let info = players.get(&USER.into()).unwrap();
    assert_eq!(
        info.game_status,
        GameStatus::Completed(GameOverStatus::Lose)
    );
    assert_eq!(info.attempts_count, 5);

    // When: check status period has come
    let result = system.run_to_block(DELAY_CHECK_STATUS_DURATION);

    // Then: Result logs are empty
    assert!(result.first().unwrap().log().is_empty());
}

#[test]
fn check_game_status_ignore_when_init_id_changed() {
    let system = init_system();
    let proxy_program = init_programs(&system).proxy_program;

    // Given:
    // - Maximum number of attempts is reached
    // - User restarts the game at block `DELAY_CHECK_STATUS_DURATION` - 1
    proxy_program.send(USER, Action::StartGame);
    system.run_next_block();
    for _ in 0..5 {
        proxy_program.send(
            USER,
            Action::CheckWord {
                word: WRONG_ANSWER.into(),
            },
        );
        system.run_next_block();
    }
    let State { players, .. } = proxy_program.read_state(0).unwrap();
    let info = players.get(&USER.into()).unwrap();
    assert_eq!(info.attempts_count, 5);

    let prev_init_id = info.init_msg_id;

    for _ in 0..DELAY_CHECK_STATUS_DURATION - 1 {
        system.run_next_block();
    }

    proxy_program.send(USER, Action::StartGame);
    system.run_next_block();
    let State { players, .. } = proxy_program.read_state(0).unwrap();
    let info = players.get(&USER.into()).unwrap();
    assert_eq!(info.game_status, GameStatus::InProgress);

    // When: check status period from previous session has come
    system.run_next_block();

    // Then: Result logs are empty and init ID's mismatch
    assert_ne!(prev_init_id, info.init_msg_id);
}

#[test]
fn check_game_status_should_declare_game_over_when_time_is_up() {
    let system = init_system();
    let proxy_program = init_programs(&system).proxy_program;

    // Given: A game is in progress
    proxy_program.send(USER, Action::StartGame);
    system.run_next_block();
    // When: Time is up
    let result = system.run_to_block(DELAY_CHECK_STATUS_DURATION);

    // Then:
    // - GameOver event is emitted
    // - The game status is set accordingly
    let log = Log::builder()
        .source(PROXY_PROGRAM)
        .dest(USER)
        .payload(Event::GameOver(GameOverStatus::Lose));
    println!("log: {:?}", log);
    let State { players, .. } = proxy_program.read_state(0).unwrap();
    let info = players.get(&USER.into()).unwrap();
    assert_eq!(info.game_status, GameStatus::InProgress);
}

fn consume_all_attempts_with_wrong_answers(program: &Program) {
    for _ in 0..5 {
        program.send(
            USER,
            Action::CheckWord {
                word: WRONG_ANSWER.into(),
            },
        );
    }
}
