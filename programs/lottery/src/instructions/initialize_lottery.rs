use crate::states::*;
use star_frame::prelude::*;

// ============================================================================
// Instruction Args
// ============================================================================

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone)]
#[borsh(crate = "star_frame::borsh")]
pub struct InitializeLotteryArgs {
    pub ticket_price: u64,     // Price per ticket in USDC lamports
    pub default_duration: i64, // Default round duration in seconds
    pub fee_bps: u16,          // Fee in basis points (e.g., 500 = 5%)
}

#[derive(InstructionArgs, BorshSerialize, BorshDeserialize, Copy, Clone, Debug)]
#[borsh(crate = "star_frame::borsh")]
pub struct InitializeLottery {
    #[ix_args(run)]
    pub args: InitializeLotteryArgs,
}

// ============================================================================
// Accounts
// ============================================================================

#[derive(AccountSet)]
pub struct InitializeLotteryAccounts {
    #[validate(funder)]
    pub admin: Signer<Mut<SystemAccount>>,

    #[validate(arg = (Create(()), Seeds(LotteryConfigSeeds)))]
    pub lottery_config: Init<Seeded<Account<LotteryConfig>>>,

    /// Treasury PDA that will receive fees
    /// CHECK: This is a PDA derived from TreasurySeeds
    pub treasury: SystemAccount,

    /// USDC mint address - we'll validate it's a valid mint
    pub usdc_mint: SystemAccount,

    pub system_program: Program<System>,
}

// ============================================================================
// Handler
// ============================================================================

#[star_frame_instruction]
fn InitializeLottery(
    accounts: &mut InitializeLotteryAccounts,
    args: InitializeLotteryArgs,
) -> Result<()> {
    // Validate fee configuration
    if args.fee_bps >= 10_000 {
        return Err(crate::SolanaLotteryPoolError::InvalidFeeConfig.into());
    }

    // Calculate bump seeds for config and treasury PDAs
    let config_seeds = LotteryConfigSeeds;
    let (_config_pda, config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &crate::SolanaLotteryPoolProgram::ID);

    let treasury_seeds = TreasurySeeds;
    let (_treasury_pda, treasury_bump) = Pubkey::find_program_address(
        &treasury_seeds.seeds(),
        &crate::SolanaLotteryPoolProgram::ID,
    );

    // Initialize lottery config
    let mut config = accounts.lottery_config.data_mut()?;
    config.admin = *accounts.admin.pubkey();
    config.treasury = *accounts.treasury.pubkey();
    config.usdc_mint = *accounts.usdc_mint.pubkey();
    config.ticket_price = args.ticket_price;
    config.default_duration = args.default_duration;
    config.fee_bps = args.fee_bps;
    config.current_round_id = 0;
    config.bump = config_bump;
    config.treasury_bump = treasury_bump;

    msg!("Lottery initialized!");
    msg!("Admin: {}", config.admin);
    msg!("Treasury: {}", config.treasury);
    msg!("USDC Mint: {}", config.usdc_mint);
    msg!("Ticket Price: {}", args.ticket_price);
    msg!("Default Duration: {} seconds", args.default_duration);
    msg!(
        "Fee: {}bps ({}%)",
        args.fee_bps,
        args.fee_bps as f64 / 100.0
    );

    Ok(())
}
