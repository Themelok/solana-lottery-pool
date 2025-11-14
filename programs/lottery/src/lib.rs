use instructions::*;
use star_frame::prelude::*;

mod errors;
mod instructions;
pub mod states;

// Re-export error type
pub use errors::SolanaLotteryPoolError;

#[cfg(test)]
mod tests;

#[derive(StarFrameProgram)]
#[program(
    instruction_set = SolanaLotteryPoolInstructionSet,
    id = "DmvwV2RNhdELbxn1nx5LVi6RexqKGDBoLJdWY9SGHjmP"
)]
pub struct SolanaLotteryPoolProgram;

#[derive(InstructionSet)]
pub enum SolanaLotteryPoolInstructionSet {
    InitializeLottery(InitializeLottery),
    OpenNextRound(OpenNextRound),
    // TODO: Add remaining lottery instructions
}
