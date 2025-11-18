/// Utility to calculate Star Frame instruction discriminators
///
/// Star Frame uses the first 8 bytes of SHA-256 hash of the instruction name
///
/// Usage:
///   cargo run --example get_discriminators

use sha2::{Digest, Sha256};

fn calculate_discriminator(instruction_name: &str) -> [u8; 8] {
    let hash = Sha256::digest(instruction_name.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash[..8]);
    discriminator
}

fn format_discriminator(disc: &[u8; 8]) -> String {
    format!(
        "[0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}]",
        disc[0], disc[1], disc[2], disc[3], disc[4], disc[5], disc[6], disc[7]
    )
}

fn main() {
    println!("🔍 Star Frame Instruction Discriminator Calculator\n");

    let instructions = vec![
        "InitializeLottery",
        "OpenNextRound",
        // Add future instructions here:
        // "BuyTicket",
        // "CloseRound",
        // "FinalizeRound",
        // "Claim",
    ];

    println!("Calculating discriminators for lottery instructions:\n");

    for instruction in instructions {
        let disc = calculate_discriminator(instruction);
        println!("{}:", instruction);
        println!("  Discriminator: {}", format_discriminator(&disc));
        println!("  Hex: {}", hex::encode(disc));
        println!();
    }

    println!("\n📝 Copy these values into your client.rs:");
    println!("   Update the discriminator constants at the top of the file\n");
}
