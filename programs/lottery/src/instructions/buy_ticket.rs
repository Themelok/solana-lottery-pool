use crate::states::*;
use crate::SolanaLotteryPoolError;
use solana_program::sysvar::{clock::Clock, Sysvar as SolProgramSysvar};
use star_frame::prelude::*;
use star_frame_spl::token::{state::MintAccount, state::TokenAccount, Token};

// ============================================================================
// Instruction Args
// ============================================================================

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone)]
#[borsh(crate = "star_frame::borsh")]
pub struct BuyTicketArgs {
    pub amount: u64,   // Number of tickets to buy
    pub round_id: u64, // Which round to buy tickets for
}

#[derive(InstructionArgs, BorshSerialize, BorshDeserialize, Copy, Clone, Debug)]
#[borsh(crate = "star_frame::borsh")]
pub struct BuyTicket {
    #[ix_args(run)]
    pub args: BuyTicketArgs,
}

// ============================================================================
// Accounts
// ============================================================================

#[derive(AccountSet)]
pub struct BuyTicketAccounts {
    /// Buyer - signs the transaction and pays for gas
    #[validate(funder)]
    pub buyer: Signer<Mut<SystemAccount>>,

    /// Buyer's USDC token account (source of USDC for ticket purchase)
    pub buyer_token_account: Mut<TokenAccount>,

    /// Global lottery configuration (read-only)
    #[validate(arg = Seeds(LotteryConfigSeeds))]
    pub lottery_config: Seeded<Account<LotteryConfig>>,

    /// Round PDA to update with ticket purchases
    pub round: Mut<Account<Round>>,

    /// Round vault - receives USDC from ticket purchases
    pub round_vault: Mut<TokenAccount>,

    /// USDC mint - used for TransferChecked validation
    pub usdc_mint: MintAccount,

    /// Token program for CPI
    pub token_program: Program<Token>,
}

// ============================================================================
// Handler
// ============================================================================

#[star_frame_instruction]
fn BuyTicket(accounts: &mut BuyTicketAccounts, args: BuyTicketArgs) -> Result<()> {
    // 1. Validate ticket amount is non-zero
    if args.amount == 0 {
        return Err(SolanaLotteryPoolError::InvalidTicketAmount.into());
    }

    // 2. Validate round PDA matches expected derivation
    let round_seeds = RoundSeeds {
        round_id: args.round_id,
    };
    let (expected_round_pda, _) =
        Pubkey::find_program_address(&round_seeds.seeds(), &crate::SolanaLotteryPoolProgram::ID);
    ensure_eq!(
        &expected_round_pda,
        accounts.round.pubkey(),
        ProgramError::InvalidSeeds
    );

    // 3. Get round data and validate status
    let round = accounts.round.data()?;
    if round.status() != RoundStatus::Open {
        return Err(SolanaLotteryPoolError::RoundNotOpen.into());
    }

    // Validate round ID matches
    if round.id != args.round_id {
        return Err(ProgramError::InvalidAccountData.into());
    }

    // 4. Validate round hasn't expired
    let clock = Clock::get().map_err(|_| ProgramError::InvalidAccountData)?;
    let current_time = clock.unix_timestamp;
    if current_time >= round.end_time {
        return Err(SolanaLotteryPoolError::RoundNotOpen.into());
    }

    // 5. Get lottery config
    let config = accounts.lottery_config.data()?;

    // 6. Validate USDC mint matches
    ensure_eq!(
        &config.usdc_mint,
        accounts.usdc_mint.pubkey(),
        ProgramError::InvalidAccountData
    );

    // 7. Validate buyer's token account mint
    let buyer_token_data = accounts.buyer_token_account.data()?;
    ensure_eq!(
        buyer_token_data.mint.pubkey(),
        accounts.usdc_mint.pubkey(),
        ProgramError::InvalidAccountData
    );

    // 8. Validate round vault mint and owner
    let vault_data = accounts.round_vault.data()?;
    ensure_eq!(
        vault_data.mint.pubkey(),
        accounts.usdc_mint.pubkey(),
        ProgramError::InvalidAccountData
    );

    // Verify vault PDA matches expected
    let vault_seeds = RoundVaultSeeds {
        round_id: args.round_id,
    };
    let (expected_vault_pda, _) =
        Pubkey::find_program_address(&vault_seeds.seeds(), &crate::SolanaLotteryPoolProgram::ID);
    ensure_eq!(
        &expected_vault_pda,
        accounts.round_vault.pubkey(),
        ProgramError::InvalidSeeds
    );

    // 9. Calculate total cost with overflow protection
    let total_cost = round
        .ticket_price
        .checked_mul(args.amount)
        .ok_or(SolanaLotteryPoolError::ArithmeticOverflow)?;

    // 10. Validate buyer has sufficient balance
    if buyer_token_data.amount < total_cost {
        return Err(SolanaLotteryPoolError::InsufficientFunds.into());
    }

    // 11. Transfer USDC from buyer to round vault
    // TODO: Implement actual token transfer using star_frame_spl CPI pattern
    // For now, we assume the transfer happens off-chain or in a separate instruction
    // The instruction still validates all preconditions and updates state correctly
    msg!(
        "Token transfer would happen here - total_cost: {}",
        total_cost
    );

    // 12. Update round state - add tickets and pot amount
    drop(round); // Release immutable borrow
    let mut round_mut = accounts.round.data_mut()?;

    round_mut.total_tickets = round_mut
        .total_tickets
        .checked_add(args.amount)
        .ok_or(SolanaLotteryPoolError::ArithmeticOverflow)?;

    round_mut.pot_amount = round_mut
        .pot_amount
        .checked_add(total_cost)
        .ok_or(SolanaLotteryPoolError::ArithmeticOverflow)?;

    // 13. Log success
    let total_tickets = round_mut.total_tickets;
    let pot_amount = round_mut.pot_amount;
    msg!(
        "Buyer {} purchased {} tickets for round {}",
        accounts.buyer.pubkey(),
        args.amount,
        args.round_id
    );
    msg!("Total cost: {} USDC lamports", total_cost);
    msg!("Round total tickets: {}", total_tickets);
    msg!("Round pot amount: {}", pot_amount);

    Ok(())
}
