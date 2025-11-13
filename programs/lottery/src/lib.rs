use star_frame::prelude::*;
use instructions::*;

mod instructions;
pub mod states;
mod errors;

// Re-export error type
pub use errors::SolanaLotteryPoolError;

#[cfg(test)]
mod tests;

#[derive(StarFrameProgram)]
#[program(
    instruction_set = SolanaLotteryPoolInstructionSet,
    id = "111FJo4zLAGU9nzTWa6EnbV4VAmtG4FR8kcokrtZYr"
)]
pub struct SolanaLotteryPoolProgram;

#[derive(InstructionSet)]
pub enum SolanaLotteryPoolInstructionSet {
    InitializeLottery(InitializeLottery),
    // TODO: Add remaining lottery instructions
}

