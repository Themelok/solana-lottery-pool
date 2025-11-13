use star_frame::prelude::*;

#[star_frame_error]
pub enum SolanaLotteryPoolError {
    #[msg("Unauthorized: signer is not the admin")]
    Unauthorized,
    #[msg("Round is not in Open status")]
    RoundNotOpen,
    #[msg("Round is not in Closed status")]
    RoundNotClosed,
    #[msg("Round has not expired yet")]
    RoundNotExpired,
    #[msg("Round is already finalized")]
    RoundAlreadyFinalized,
    #[msg("Invalid ticket amount (must be > 0)")]
    InvalidTicketAmount,
    #[msg("No tickets were sold in this round")]
    NoTicketsSold,
    #[msg("Randomness is not available")]
    RandomnessNotAvailable,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Invalid fee configuration (fee_bps must be < 10000)")]
    InvalidFeeConfig,
    #[msg("Insufficient funds in buyer's account")]
    InsufficientFunds,
    #[msg("Token transfer failed")]
    TokenTransferFailed,
}
