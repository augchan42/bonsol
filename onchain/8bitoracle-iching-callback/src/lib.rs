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
        system_instruction,
        system_program::ID as SYS_ID,
        sysvar::{clock::Clock, Sysvar},
        rent::Rent,
        program::invoke_signed,
    },
    thiserror::Error,
};

// Add this line to declare the program ID
solana_program::declare_id!("6yx54uNRbsyCzfwLyKEC5xuzjiN7jW5tovmMgGWkmk2m");

// The expected image ID for our 8BitOracle I Ching program
pub const BITORACLE_ICHING_IMAGE_ID: &str = "83fd7b6a7011b7b842f9ddc83dc7c470a0d4fb71fb6c8dd3064387bac21fd8de";
pub const ASCII_ART_SIZE: usize = 47; // Fixed size for ASCII art
pub const CALLBACK_VERSION: &str = "v0.1.4";

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CallbackInstruction {
    /// Initialize a new storage account
    /// Accounts expected:
    /// 1. `[signer]` The account paying for rent
    /// 2. `[writable]` The storage account to initialize
    /// 3. `[]` The system program
    Initialize,
    
    /// Process the callback from Bonsol
    /// Accounts expected by handle_callback
    Callback(Vec<u8>),
}

#[derive(Error, Debug)]
pub enum CallbackError {
    #[error("Invalid instruction data")]
    InvalidInstruction,
    #[error("Invalid hexagram data")]
    InvalidHexagramData,
    #[error("Invalid ASCII art length")]
    InvalidAsciiArtLength,
    #[error("Insufficient accounts provided")]
    InsufficientAccounts,
    #[error("Account not rent exempt")]
    NotRentExempt,
    #[error("Account too small")]
    AccountTooSmall,
    #[error("Invalid instruction data")]
    InvalidInstructionData,
}

impl From<CallbackError> for ProgramError {
    fn from(e: CallbackError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct HexagramData {
    pub lines: [u8; 6],                // The 6,7,8,9 values for each line
    pub ascii_art: [u8; ASCII_ART_SIZE], // The ASCII representation as fixed-size array
    pub timestamp: i64,                // When the reading was done
    pub is_initialized: bool,          // To check if the account is initialized
}

entrypoint!(process);

pub fn process(pid: &Pubkey, accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    msg!("🎲 8BitOracle I Ching {} - Processing Start", CALLBACK_VERSION);
    msg!("📊 Program ID: {}", pid);
    msg!("📝 Number of accounts: {}", accs.len());
    msg!("📦 Input data length: {}", data.len());
    
    // Log raw instruction data for debugging
    msg!("📦 Raw instruction data: {:?}", data);
    
    // Try to deserialize as CallbackInstruction first
    match CallbackInstruction::try_from_slice(data) {
        Ok(instruction) => {
            match instruction {
                CallbackInstruction::Initialize => {
                    msg!("Processing Initialize instruction");
                    if accs.len() != 3 {
                        msg!("❌ Error: Initialize requires exactly 3 accounts");
                        return Err(CallbackError::InsufficientAccounts.into());
                    }
                    
                    let payer_account = &accs[0];
                    let storage_account = &accs[1];
                    let system_program = &accs[2];
                    
                    if !payer_account.is_signer {
                        msg!("❌ Error: Payer must be a signer");
                        return Err(ProgramError::MissingRequiredSignature);
                    }
                    
                    if !storage_account.is_writable {
                        msg!("❌ Error: Storage account must be writable");
                        return Err(ProgramError::InvalidAccountData);
                    }
                    
                    if system_program.key != &SYS_ID {
                        msg!("❌ Error: Invalid system program");
                        return Err(ProgramError::InvalidAccountData);
                    }

                    // Calculate required space and rent
                    let required_size = std::mem::size_of::<HexagramData>();
                    let rent = Rent::get()?;
                    let lamports = rent.minimum_balance(required_size);

                    msg!("Creating storage account...");
                    msg!("Required space: {} bytes", required_size);
                    msg!("Required lamports: {}", lamports);

                    // Find PDA seeds
                    let seeds = &[
                        b"hexagram",
                        payer_account.key.as_ref(),
                    ];
                    let (pda, bump_seed) = Pubkey::find_program_address(seeds, pid);

                    // Verify the derived PDA matches our storage account
                    if pda != *storage_account.key {
                        msg!("❌ Error: Storage account does not match PDA");
                        msg!("Expected: {}", pda);
                        msg!("Got: {}", storage_account.key);
                        return Err(ProgramError::InvalidArgument);
                    }

                    // Create the storage account using invoke_signed
                    let create_account_ix = system_instruction::create_account(
                        payer_account.key,
                        storage_account.key,
                        lamports,
                        required_size as u64,
                        pid,
                    );

                    let signer_seeds = &[
                        b"hexagram",
                        payer_account.key.as_ref(),
                        &[bump_seed],
                    ];

                    invoke_signed(
                        &create_account_ix,
                        &[
                            payer_account.clone(),
                            storage_account.clone(),
                            system_program.clone(),
                        ],
                        &[signer_seeds],
                    )?;
                    
                    // Initialize the account data
                    msg!("Initializing storage account data...");
                    let hexagram = HexagramData {
                        lines: [0u8; 6],
                        ascii_art: [0u8; ASCII_ART_SIZE],
                        timestamp: 0,
                        is_initialized: true,
                    };
                    
                    hexagram.serialize(&mut &mut storage_account.try_borrow_mut_data()?[..])?;
                    msg!("✓ Storage account initialized successfully");
                    Ok(())
                },
                CallbackInstruction::Callback(callback_data) => {
                    msg!("Processing Callback instruction");
                    process_callback(pid, accs, &callback_data)
                }
            }
        },
        Err(_) => {
            // If deserialization fails, assume it's a raw callback from Bonsol
            msg!("Processing raw callback data");
            process_callback(pid, accs, data)
        }
    }
}

pub fn process_callback(pid: &Pubkey, accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    msg!("🎲 8BitOracle I Ching {} - Processing Start", CALLBACK_VERSION);
    msg!("📊 Program ID: {}", pid);
    msg!("📝 Number of accounts: {}", accs.len());
    msg!("📦 Input data length: {}", data.len());
    
    // Log all account info
    for (i, acc) in accs.iter().enumerate() {
        msg!("Account #{} Details:", i);
        msg!("  Key: {}", acc.key);
        msg!("  Owner: {}", acc.owner);
        msg!("  Lamports: {}", acc.lamports());
        msg!("  Data Length: {}", acc.data_len());
        msg!("  Executable: {}", acc.executable);
        msg!("  Rent Epoch: {}", acc.rent_epoch);
        msg!("  Is Signer: {}", acc.is_signer);
        msg!("  Is Writable: {}", acc.is_writable);
    }
    
    // We need at least 3 accounts:
    // 0. Execution account (for validation)
    // 1. Bonsol program account
    // 2. Storage account (for hexagram data)
    if accs.len() < 3 {
        msg!("❌ Error: Insufficient accounts. Need at least 3 accounts (execution, bonsol, and storage)");
        return Err(CallbackError::InsufficientAccounts.into());
    }
    
    // Find our storage account - it should be the one owned by our program
    let storage_account = accs.iter().find(|acc| acc.owner == pid)
        .ok_or_else(|| {
            msg!("❌ Error: Could not find storage account owned by our program");
            ProgramError::IncorrectProgramId
        })?;
    
    // Verify the storage account is writable
    if !storage_account.is_writable {
        msg!("❌ Error: Storage account must be writable");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Verify the storage account is rent exempt
    let rent = Rent::get()?;
    if !rent.is_exempt(storage_account.lamports(), storage_account.data_len()) {
        msg!("❌ Error: Storage account must be rent exempt");
        return Err(CallbackError::NotRentExempt.into());
    }
    
    // Verify the storage account is large enough
    let required_size = std::mem::size_of::<HexagramData>();
    if storage_account.data_len() < required_size {
        msg!("❌ Error: Storage account too small");
        msg!("  Required size: {}", required_size);
        msg!("  Account size: {}", storage_account.data_len());
        return Err(CallbackError::AccountTooSmall.into());
    }
    
    // Log the raw data for debugging
    msg!("Raw callback data: {:?}", data);
    
    msg!("🔄 Processing callback...");
    let cb_data: BonsolCallback = handle_callback(BITORACLE_ICHING_IMAGE_ID, &accs[0].key, accs, data)?;
    msg!("✓ Callback processed successfully");
    msg!("📦 Input digest length: {}", cb_data.input_digest.len());
    msg!("📦 Committed outputs length: {}", cb_data.committed_outputs.len());
    
    let out = &cb_data.committed_outputs;
    msg!("🔍 Validating output data:");
    msg!("  Output length: {}", out.len());
    if out.len() > 0 {
        msg!("  First byte: 0x{:02x}", out[0]);
    }
    
    // Look for our marker byte (0xaa) in the output
    let marker_pos = out.iter().position(|&x| x == 0xaa)
        .ok_or_else(|| {
            msg!("❌ Error: Could not find marker byte 0xaa in output");
            CallbackError::InvalidHexagramData
        })?;
    
    msg!("Found marker byte at position {}", marker_pos);
    
    // Ensure we have enough data after the marker
    if out.len() < marker_pos + 54 {
        msg!("❌ Error: Insufficient data after marker");
        msg!("  Available: {} bytes", out.len() - marker_pos);
        msg!("  Required: 54 bytes");
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Extract line values (6 bytes after marker)
    let mut lines = [0u8; 6];
    lines.copy_from_slice(&out[marker_pos + 1..marker_pos + 7]);
    msg!("📊 Hexagram lines: {:?}", lines);
    
    if !lines.iter().all(|&x| (6..=9).contains(&x)) {
        msg!("❌ Error: Invalid line values");
        msg!("  Values must be between 6 and 9");
        msg!("  Got: {:?}", lines);
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    msg!("🎨 Processing ASCII art...");
    let mut ascii_art = [0u8; ASCII_ART_SIZE];
    let ascii_slice = &out[marker_pos + 7..marker_pos + 7 + ASCII_ART_SIZE];
    msg!("  ASCII slice length: {}", ascii_slice.len());
    
    if ascii_slice.len() != ASCII_ART_SIZE {
        msg!("❌ Error: Invalid ASCII art length");
        msg!("  Expected: {}", ASCII_ART_SIZE);
        msg!("  Got: {}", ascii_slice.len());
        return Err(CallbackError::InvalidAsciiArtLength.into());
    }
    
    ascii_art.copy_from_slice(ascii_slice);
    msg!("✓ ASCII art processed successfully");
    
    // Log the ASCII art visualization
    msg!("Hexagram visualization (bottom to top):");
    let ascii_str = std::str::from_utf8(&ascii_art).unwrap_or("Invalid UTF-8");
    for line in ascii_str.lines() {
        msg!("  {}", line);
    }
    
    // Get timestamp directly from sysvar without needing account
    let timestamp = Clock::get()?.unix_timestamp;
    msg!("⏰ Timestamp: {}", timestamp);
    
    let hexagram = HexagramData {
        lines,
        ascii_art,
        timestamp,
        is_initialized: true,
    };
    
    msg!("💾 Storing hexagram data...");
    let mut storage_data = storage_account.try_borrow_mut_data()?;
    hexagram.serialize(&mut &mut storage_data[..])?;
    
    msg!("✨ Hexagram processed and stored successfully");
    msg!("  Lines: {:?}", hexagram.lines);
    msg!("  Timestamp: {}", hexagram.timestamp);
    msg!("  Is initialized: {}", hexagram.is_initialized);
    
    Ok(())
} 