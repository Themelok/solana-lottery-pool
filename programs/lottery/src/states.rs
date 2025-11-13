use crate::SolanaLotteryPoolError;
use star_frame::prelude::*;

// ============================================================================
// RoundStatus Enum
// ============================================================================

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, borsh::BorshSerialize, borsh::BorshDeserialize)]
#[borsh(use_discriminant = true)]
pub enum RoundStatus {
    Open = 0,      // Accepting tickets
    Closed = 1,    // Stopped, awaiting finalization
    Finalized = 2, // Winner selected, pot distributed
}

impl Default for RoundStatus {
    fn default() -> Self {
        RoundStatus::Open
    }
}

// ============================================================================
// LotteryConfig - Global Configuration PDA
// ============================================================================

#[derive(Debug, GetSeeds, Clone)]
#[get_seeds(seed_const = b"LOTTERY_CONFIG")]
pub struct LotteryConfigSeeds;

#[zero_copy(pod)]
#[derive(Default, Debug, Eq, PartialEq, ProgramAccount)]
#[program_account(seeds = LotteryConfigSeeds)]
pub struct LotteryConfig {
    pub admin: Pubkey,              // Authority who can manage the lottery
    pub treasury: Pubkey,           // PDA that receives fees
    pub usdc_mint: Pubkey,          // USDC SPL token mint address
    pub ticket_price: u64,          // Price per ticket in USDC lamports (e.g., 1_000_000 = 1 USDC)
    pub default_duration: i64,      // Default round duration in seconds
    pub fee_bps: u16,               // Fee in basis points (e.g., 500 = 5%)
    pub current_round_id: u64,      // Incrementing round counter
    pub bump: u8,                   // PDA bump seed
    pub treasury_bump: u8,          // Treasury PDA bump seed
    pub _padding: [u8; 6],          // Padding for alignment
}

// ============================================================================
// Round - Per-Round State PDA
// ============================================================================

#[derive(Debug, GetSeeds, Clone)]
#[get_seeds(seed_const = b"ROUND")]
pub struct RoundSeeds {
    pub round_id: u64,
}

#[zero_copy(pod)]
#[derive(Default, Debug, Eq, PartialEq, ProgramAccount)]
#[program_account(seeds = RoundSeeds)]
pub struct Round {
    pub id: u64,                    // Round identifier
    pub start_time: i64,            // Unix timestamp when round opened
    pub end_time: i64,              // Unix timestamp when round should close
    pub ticket_price: u64,          // Snapshot of ticket price for this round
    pub total_tickets: u64,         // Total number of tickets sold
    pub pot_amount: u64,            // Total USDC in the pot (lamports)
    pub winner: Pubkey,             // Winner's pubkey (zero if not finalized)
    pub winner_ticket_index: u64,   // Index of winning ticket (0 if not finalized)
    pub randomness: u64,            // Random value used for winner selection (0 if not available)
    pub status: u8,                 // RoundStatus as u8
    pub vault_bump: u8,             // Bump for round vault PDA
    pub bump: u8,                   // Bump for round PDA
    pub _padding: [u8; 5],          // Padding for alignment
}

impl Round {
    pub fn status(&self) -> RoundStatus {
        match self.status {
            0 => RoundStatus::Open,
            1 => RoundStatus::Closed,
            2 => RoundStatus::Finalized,
            _ => RoundStatus::Open, // Default fallback
        }
    }

    pub fn set_status(&mut self, status: RoundStatus) {
        self.status = status as u8;
    }
}

// ============================================================================
// Treasury Seeds
// ============================================================================

#[derive(Debug, GetSeeds, Clone)]
#[get_seeds(seed_const = b"TREASURY")]
pub struct TreasurySeeds;

// ============================================================================
// Round Vault Seeds (Associated Token Account for Round PDA)
// ============================================================================

#[derive(Debug, GetSeeds, Clone)]
#[get_seeds(seed_const = b"ROUND_VAULT")]
pub struct RoundVaultSeeds {
    pub round_id: u64,
}

// ============================================================================
// Validation Traits
// ============================================================================

// Validate that a signer is the admin
pub struct AdminAuthority(pub Pubkey);

impl AccountValidate<&Pubkey> for LotteryConfig {
    fn validate_account(self_ref: &Self::Ptr, signer: &Pubkey) -> Result<()> {
        ensure_eq!(
            &self_ref.admin,
            signer,
            SolanaLotteryPoolError::Unauthorized
        );
        Ok(())
    }
}
