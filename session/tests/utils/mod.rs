#![allow(warnings)]
use gstd::ActorId;
use gtest::{Program, ProgramBuilder, System};
use session_io::Event;

pub const PROXY_PROGRAM: u64 = 1;
pub const TARGET_PROGRAM: u64 = 2;

pub const USER: u64 = 3;

pub const WRONG_ANSWER: &str = "human"; // item at index 0 or 1 in `BANK_OF_WORDS` is the wrong answer
pub const CORRECT_ANSWER: &str = "horse"; // item at index 2 in `BANK_OF_WORDS` is the correct answer

pub struct ProgramPair<'a> {
    pub target_program: Program<'a>,
    pub proxy_program: Program<'a>,
}

pub fn init_system() -> System {
    let system = System::new();
    system.init_logger();
    system
}

pub fn init_programs(sys: &System) -> ProgramPair<'_> {
    let proxy_program = Program::current(sys);
    let target_program =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(TARGET_PROGRAM)
            .build(sys);
    sys.mint_to(USER, 100000000000000000000);
    let result = target_program.send_bytes(USER, []);
    sys.run_next_block();
    let result = proxy_program.send::<_, ActorId>(USER, TARGET_PROGRAM.into());
    sys.run_next_block();
    ProgramPair {
        target_program,
        proxy_program,
    }
}

pub fn final_panic_message(message: &str) -> String {
    println!("Reverts with: {message}");
    "Panic occurred: panicked with '<unknown>'".into()
}

pub fn word_checked_on_wrong_answer_event() -> Event {
    use gstd::collections::HashSet;

    let mut correct_positions: Vec<u8> = Vec::new();
    let mut contained_in_word: Vec<u8> = Vec::new();
    let mut used_indices = HashSet::new();

    for (i, (s_char, u_char)) in CORRECT_ANSWER.chars().zip(WRONG_ANSWER.chars()).enumerate() {
        if s_char == u_char {
            correct_positions.push(i.try_into().unwrap());
            used_indices.insert(i);
        }
    }

    for (i, u_char) in WRONG_ANSWER.chars().enumerate() {
        if !used_indices.contains(&i) && CORRECT_ANSWER.contains(u_char) {
            let mut added = false;
            for (j, s_char) in CORRECT_ANSWER.chars().enumerate() {
                if s_char == u_char && !used_indices.contains(&j) {
                    contained_in_word.push(i.try_into().unwrap());
                    used_indices.insert(j);
                    added = true;
                    break;
                }
            }
            let i: u8 = i.try_into().unwrap();
            if !added && !correct_positions.contains(&i) && !contained_in_word.contains(&i) {
                contained_in_word.push(i);
            }
        }
    }

    Event::WordChecked {
        correct_positions,
        contained_in_word,
    }
}
