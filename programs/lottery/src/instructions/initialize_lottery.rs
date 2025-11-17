use crate::states::*;
use star_frame::account_set::modifiers::CanInitAccount;
use star_frame::prelude::*;
use star_frame_spl::associated_token::state::{AssociatedTokenAccount, InitAta};
use star_frame_spl::associated_token::AssociatedToken;
use star_frame_spl::token::{state::MintAccount, Token};

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

    /// USDC mint address - validated as a proper SPL token mint
    pub usdc_mint: MintAccount,

    /// Treasury PDA - owner of the ATA (no data stored here)
    pub treasury_pda: Mut<SystemAccount>,

    /// Treasury ATA - will be created in handler using InitAta pattern if needed
    pub treasury_ata: AssociatedTokenAccount,

    pub system_program: Program<System>,
    pub token_program: Program<Token>,
    pub associated_token_program: Program<AssociatedToken>,
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

    // Derive treasury PDA and verify it matches the provided account
    let treasury_seeds = TreasurySeeds;
    let (treasury_pda, treasury_bump) = Pubkey::find_program_address(
        &treasury_seeds.seeds(),
        &crate::SolanaLotteryPoolProgram::ID,
    );

    ensure_eq!(
        &treasury_pda,
        accounts.treasury_pda.pubkey(),
        ProgramError::InvalidSeeds
    );

    // Check if treasury ATA needs to be created
    let treasury_ata_info = accounts.treasury_ata.account_info();
    if treasury_ata_info.lamports() == 0 || treasury_ata_info.data_is_empty() {
        msg!("Creating treasury ATA using InitAta pattern...");

        // Create InitAta helper
        let init_ata = InitAta {
            wallet: &accounts.treasury_pda,
            mint: &accounts.usdc_mint,
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        };

        // Use the CanInitAccount trait to create the ATA
        // Pass InitAta as the argument - the framework handles the rest
        // IF_NEEDED = false because we already checked it's empty
        accounts.treasury_ata.init_account::<false>(
            init_ata,
            None, // No seeds needed for ATA (derived from wallet + mint)
            &Context::new(&crate::SolanaLotteryPoolProgram::ID),
        )?;

        msg!("Treasury ATA created successfully");
    }

    // Calculate config PDA bump
    let config_seeds = LotteryConfigSeeds;
    let (_config_pda, config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &crate::SolanaLotteryPoolProgram::ID);

    // Initialize lottery config
    let mut config = accounts.lottery_config.data_mut()?;
    config.admin = *accounts.admin.pubkey();
    config.treasury = *accounts.treasury_ata.pubkey(); // Store the ATA address
    config.usdc_mint = *accounts.usdc_mint.pubkey();
    config.ticket_price = args.ticket_price;
    config.default_duration = args.default_duration;
    config.fee_bps = args.fee_bps;
    config.current_round_id = 0;
    config.bump = config_bump;
    config.treasury_bump = treasury_bump;

    msg!("Lottery initialized with ATA!");
    msg!("Admin: {}", config.admin);
    msg!("Treasury PDA: {}", treasury_pda);
    msg!("Treasury ATA: {}", config.treasury);
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
