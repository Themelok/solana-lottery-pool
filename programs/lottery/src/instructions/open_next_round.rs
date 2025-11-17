use crate::states::*;
use solana_program::sysvar::{clock::Clock, Sysvar as SolProgramSysvar};
use star_frame::prelude::*;
use star_frame_spl::token::{state::MintAccount, state::TokenAccount, Token};

// ============================================================================
// Instruction Args
// ============================================================================

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone)]
#[borsh(crate = "star_frame::borsh")]
pub struct OpenNextRoundArgs {
    pub duration_seconds: i64, // How long the round should run
    pub round_id: u64,         // Expected round ID (must match current_round_id + 1)
}

#[derive(InstructionArgs, BorshSerialize, BorshDeserialize, Copy, Clone, Debug)]
#[borsh(crate = "star_frame::borsh")]
pub struct OpenNextRound {
    #[ix_args(run)]
    pub args: OpenNextRoundArgs,
}

// ============================================================================
// Accounts
// ============================================================================

#[derive(AccountSet)]
pub struct OpenNextRoundAccounts {
    /// Admin who can open rounds - must match lottery_config.admin
    #[validate(funder)]
    pub admin: Signer<Mut<SystemAccount>>,

    /// Global lottery configuration - needs to be mutable to increment round counter
    #[validate(arg = Seeds(LotteryConfigSeeds))]
    pub lottery_config: Mut<Seeded<Account<LotteryConfig>>>,

    /// New round PDA to be created - seeds verified in handler
    #[validate(arg = (Create(()), Seeds(RoundSeeds { round_id: self.get_next_round_id() })))]
    pub round: Init<Seeded<Account<Round>>>,

    /// Round vault - token account that will hold USDC during this round
    pub round_vault: TokenAccount,

    /// USDC mint - must match lottery_config
    pub usdc_mint: MintAccount,

    /// System program for account creation
    pub system_program: Program<System>,

    /// Token program for token account creation
    pub token_program: Program<Token>,
}
impl OpenNextRoundAccounts {
    fn get_next_round_id(&self) -> u64 {
        self.lottery_config
            .data()
            .map(|config| config.current_round_id + 1)
            .unwrap_or(0)
    }
}
// ============================================================================
// Handler
// ============================================================================

#[star_frame_instruction]
fn OpenNextRound(accounts: &mut OpenNextRoundAccounts, args: OpenNextRoundArgs) -> Result<()> {
    // Validate duration is positive
    if args.duration_seconds <= 0 {
        return Err(ProgramError::InvalidArgument.into());
    }

    // Get lottery config data
    let config = accounts.lottery_config.data()?;

    // Validate admin authority
    ensure_eq!(
        &config.admin,
        accounts.admin.pubkey(),
        crate::SolanaLotteryPoolError::Unauthorized
    );

    // Validate USDC mint matches
    if config.usdc_mint != *accounts.usdc_mint.pubkey() {
        return Err(ProgramError::InvalidAccountData.into());
    }

    // Verify that args.round_id matches the expected next round ID
    let expected_next_round_id = config
        .current_round_id
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if args.round_id != expected_next_round_id {
        msg!(
            "Invalid round_id: expected {}, got {}",
            expected_next_round_id,
            args.round_id
        );
        return Err(ProgramError::InvalidArgument.into());
    }

    // Calculate round vault bump
    let vault_seeds = RoundVaultSeeds {
        round_id: args.round_id,
    };
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&vault_seeds.seeds(), &crate::SolanaLotteryPoolProgram::ID);

    // Verify vault PDA matches the provided account
    ensure_eq!(
        &vault_pda,
        accounts.round_vault.pubkey(),
        ProgramError::InvalidSeeds
    );

    // Validate round vault exists and is properly initialized
    let vault_data = accounts.round_vault.data()?;
    ensure_eq!(
        vault_data.mint.pubkey(),
        accounts.usdc_mint.pubkey(),
        ProgramError::InvalidAccountData
    );
    ensure_eq!(&vault_data.owner, &vault_pda, ProgramError::IllegalOwner);

    // Get round PDA and verify it matches
    let round_seeds = RoundSeeds {
        round_id: args.round_id,
    };
    let (expected_round_pda, round_bump) =
        Pubkey::find_program_address(&round_seeds.seeds(), &crate::SolanaLotteryPoolProgram::ID);

    ensure_eq!(
        &expected_round_pda,
        accounts.round.pubkey(),
        ProgramError::InvalidSeeds
    );

    // Get current timestamp
    let clock = Clock::get().map_err(|_| ProgramError::InvalidAccountData)?;
    let start_time = clock.unix_timestamp;
    let end_time = start_time
        .checked_add(args.duration_seconds)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Initialize round data (Star Frame handles account creation via Init<Seeded<>>)
    let mut round = accounts.round.data_mut()?;
    round.id = args.round_id;
    round.start_time = start_time;
    round.end_time = end_time;
    round.ticket_price = config.ticket_price;
    round.total_tickets = 0;
    round.pot_amount = 0;
    round.winner = Pubkey::default();
    round.winner_ticket_index = 0;
    round.randomness = 0;
    round.set_status(RoundStatus::Open);
    round.vault_bump = vault_bump;
    round.bump = round_bump;

    // Update lottery config with new round ID
    drop(config);
    let mut config_mut = accounts.lottery_config.data_mut()?;
    config_mut.current_round_id = args.round_id;

    let ticket_price_value = round.ticket_price;

    msg!("Round {} opened!", args.round_id);
    msg!("Start time: {}", start_time);
    msg!("End time: {}", end_time);
    msg!("Duration: {} seconds", args.duration_seconds);
    msg!("Ticket price: {} USDC lamports", ticket_price_value);
    msg!("Round vault: {}", accounts.round_vault.pubkey());

    Ok(())
}
