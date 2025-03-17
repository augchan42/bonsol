use {
    bonsol_interface::callback::{handle_callback, BonsolCallback},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program::ID as SYS_ID,
        sysvar::{clock::Clock, Sysvar},
    },
    thiserror::Error,
};

// The expected image ID for our 8BitOracle I Ching program
pub const BITORACLE_ICHING_IMAGE_ID: &str = "83fd7b6a7011b7b842f9ddc83dc7c470a0d4fb71fb6c8dd3064387bac21fd8de";
pub const ASCII_ART_SIZE: usize = 47; // Fixed size for ASCII art

// Add version constant
pub const CALLBACK_VERSION: &str = "v0.1.4"; // Increment this each time we deploy

#[derive(Error, Debug)]
pub enum CallbackError {
    #[error("Invalid instruction data")]
    InvalidInstruction,
    #[error("Account not rent exempt")]
    NotRentExempt,
    #[error("Invalid hexagram data")]
    InvalidHexagramData,
    #[error("Invalid signer")]
    InvalidSigner,
    #[error("Invalid account owner")]
    InvalidOwner,
    #[error("Invalid system program")]
    InvalidSystemProgram,
    #[error("Invalid ASCII art length")]
    InvalidAsciiArtLength,
}

impl From<CallbackError> for ProgramError {
    fn from(e: CallbackError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// Structure to store hexagram data on-chain
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct HexagramData {
    pub lines: [u8; 6],                // The 6,7,8,9 values for each line
    pub ascii_art: [u8; ASCII_ART_SIZE], // The ASCII representation as fixed-size array
    pub timestamp: i64,                // When the reading was done
    pub is_initialized: bool,          // To check if the account is initialized
}

entrypoint!(process);

pub fn process(pid: &Pubkey, accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    solana_program::msg!("8BitOracle I Ching {} - Start", CALLBACK_VERSION);
    
    let stripped = data.get(1..).ok_or(CallbackError::InvalidInstruction)?;
    solana_program::msg!("Processing callback with {} accounts", accs.len());
    
    let cb_data: BonsolCallback = handle_callback(BITORACLE_ICHING_IMAGE_ID, &accs[0].key, accs, stripped)?;
    solana_program::msg!("Callback processed, validating outputs");
    
    let sys = &accs[5];
    if !sys.executable || sys.key != &SYS_ID {
        solana_program::msg!("Invalid system program: {}", sys.key);
        return Err(CallbackError::InvalidSystemProgram.into());
    }
    
    let out = &cb_data.committed_outputs;
    if out.len() != 54 || out[0] != 0xaa {
        solana_program::msg!("Invalid output size or marker");
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    let mut lines = [0u8; 6];
    lines.copy_from_slice(&out[1..7]);
    
    if !lines.iter().all(|&x| (6..=9).contains(&x)) {
        solana_program::msg!("Invalid line values: {:?}", lines);
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Convert ASCII art to fixed-size array
    let mut ascii_art = [0u8; ASCII_ART_SIZE];
    let ascii_slice = &out[7..];
    if ascii_slice.len() != ASCII_ART_SIZE {
        solana_program::msg!("Invalid ASCII art length: {}", ascii_slice.len());
        return Err(CallbackError::InvalidAsciiArtLength.into());
    }
    ascii_art.copy_from_slice(ascii_slice);
    
    let hexagram = HexagramData {
        lines,
        ascii_art,
        timestamp: Clock::get()?.unix_timestamp,
        is_initialized: true,
    };
    
    solana_program::msg!("Hexagram processed successfully");
    Ok(())
} 