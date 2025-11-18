# Client Example

This directory contains example client code for interacting with the Solana Lottery Pool program.

## Overview

The `client.rs` example demonstrates how to:
- Initialize the lottery program
- Open new lottery rounds
- Derive PDAs (Program Derived Addresses)
- Build and send transactions

## Prerequisites

Before running the client, you need:

1. **Solana CLI tools** installed
2. **A funded devnet wallet** (get SOL from [Solana Faucet](https://faucet.solana.com/))
3. **The program deployed** to devnet/localnet
4. **Instruction discriminators** from the deployed program

## Getting Instruction Discriminators

Star Frame uses 8-byte discriminators derived from instruction names. To get the correct discriminators:

### Method 1: From IDL (Recommended)

```bash
# Generate the IDL
cd programs/lottery
cargo test --features idl generate_idl

# The IDL will be in target/idl/solana_lottery_pool.json
# Look for the "discriminator" field in each instruction
```

The discriminators are in the IDL under each instruction definition.

### Method 2: From Program Logs

When you call an instruction and it fails with "unknown instruction", the logs will show what discriminator was sent. You can also inspect successful transactions to see the discriminators.

### Method 3: Calculate from Source

Star Frame discriminators are the first 8 bytes of the SHA-256 hash of the instruction name:

```rust
use sha2::{Sha256, Digest};

fn get_discriminator(name: &str) -> [u8; 8] {
    let hash = Sha256::digest(name.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

// Example:
// get_discriminator("InitializeLottery")
// get_discriminator("OpenNextRound")
```

## Building the Example

Since the lottery program uses `crate-type = ["cdylib"]`, you'll need to set up the example separately:

### Option 1: Standalone Build

Create a separate binary crate:

```bash
cd examples
cargo init --name lottery-client
# Copy client.rs into src/main.rs
# Update Cargo.toml with dependencies (see below)
cargo build --release
```

### Option 2: Add to Workspace

Update `programs/lottery/Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # Add rlib
```

Then add to the `[[example]]` section:

```toml
[[example]]
name = "client"
path = "../../examples/client.rs"
```

## Required Dependencies

Add these to your client's `Cargo.toml`:

```toml
[dependencies]
borsh = "1.5.7"
solana-client = "2.1"
solana-sdk = "2.1"
spl-token = "6.0"
spl-associated-token-account = "6.0"
```

## Configuration

Update the constants in `client.rs`:

```rust
// Update with your deployed program ID
const PROGRAM_ID: &str = "YOUR_DEPLOYED_PROGRAM_ID";

// Update discriminators from IDL or calculation
const INITIALIZE_LOTTERY_DISCRIMINATOR: [u8; 8] = [/* your values */];
const OPEN_NEXT_ROUND_DISCRIMINATOR: [u8; 8] = [/* your values */];
```

## Running the Example

1. **Set up your wallet:**

```bash
# Generate a new keypair or use existing
solana-keygen new -o ~/.config/solana/lottery-admin.json

# Get devnet SOL
solana airdrop 2 ~/.config/solana/lottery-admin.json --url devnet
```

2. **Update the code to load your keypair:**

```rust
// In main(), replace:
let payer = Keypair::new();

// With:
let payer = solana_sdk::signature::read_keypair_file(
    std::env::var("HOME").unwrap() + "/.config/solana/lottery-admin.json"
).expect("Failed to load keypair");
```

3. **Uncomment the instruction calls in `main()`:**

```rust
// Uncomment these lines:
client.initialize_lottery(
    &usdc_mint,
    1_000_000,  // 1 USDC
    3600,       // 1 hour
    500,        // 5%
)?;

client.open_next_round(
    &usdc_mint,
    1,      // Round 1
    86400,  // 24 hours
)?;
```

4. **Run the client:**

```bash
cargo run --example client
# or if standalone:
cargo run
```

## Testing Locally

For local testing with `solana-test-validator`:

1. **Start local validator:**

```bash
solana-test-validator
```

2. **Deploy the program:**

```bash
# In programs/lottery
cargo build-sbf
solana program deploy target/deploy/solana_lottery_pool.so --url localhost
```

3. **Update client with localhost:**

```rust
let rpc_url = "http://localhost:8899";
```

4. **Create a local USDC mint or use Devnet USDC**

## Troubleshooting

### "Unknown instruction" error

- Verify discriminators match the deployed program
- Check that the program ID is correct
- Ensure accounts are in the correct order

### "Account not found" errors

- Make sure you've funded your wallet with SOL
- For initialize, ensure you haven't already initialized
- For open_round, ensure lottery is initialized first

### "Invalid seeds" errors

- Double-check PDA derivation matches program code
- Ensure seed constants (like "LOTTERY_CONFIG") match exactly

## Next Steps

After successfully running the example:

1. Implement the remaining instructions (BuyTicket, CloseRound, FinalizeRound)
2. Add error handling and retry logic
3. Create a proper CLI tool or web interface
4. Set up monitoring and logging

## Additional Resources

- [Star Frame Documentation](https://docs.rs/star_frame/latest/star_frame/)
- [Solana Cookbook](https://solanacookbook.com/)
- [SPL Token Documentation](https://spl.solana.com/token)
