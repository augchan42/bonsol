use crate::{actions::*, error::ChannelError};
use bonsol_interface::bonsol_schema::{parse_ix_data, ChannelInstructionIxType};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey, msg};

#[inline]
pub fn program<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &'a [u8],
) -> ProgramResult {
    msg!("🔍 Bonsol Program: Processing instruction");
    msg!("Instruction data length: {} bytes", instruction_data.len());
    msg!("Raw instruction data: {:?}", instruction_data);
    msg!("Number of accounts: {}", accounts.len());
    
    for (i, account) in accounts.iter().enumerate() {
        msg!("Account {}: {}", i, account.key);
        msg!("  Is Signer: {}", account.is_signer);
        msg!("  Is Writable: {}", account.is_writable);
    }
    
    msg!("Attempting to parse instruction data...");
    let ix = match parse_ix_data(instruction_data) {
        Ok(parsed) => {
            msg!("✓ Successfully parsed instruction data");
            parsed
        }
        Err(e) => {
            msg!("❌ Failed to parse instruction data: {:?}", e);
            return Err(ChannelError::InvalidInstructionParse.into());
        }
    };
    
    msg!("Instruction type: {:?}", ix.ix_type());
    match ix.ix_type() {
        ChannelInstructionIxType::ClaimV1 => {
            msg!("Processing ClaimV1 instruction");
            process_claim_v1(accounts, ix)?;
        }
        ChannelInstructionIxType::DeployV1 => {
            msg!("Processing DeployV1 instruction");
            process_deploy_v1(accounts, ix)?;
        }
        ChannelInstructionIxType::ExecuteV1 => {
            msg!("Processing ExecuteV1 instruction");
            process_execute_v1(accounts, ix)?;
        }
        ChannelInstructionIxType::StatusV1 => {
            msg!("Processing StatusV1 instruction");
            process_status_v1(accounts, ix)?;
        }
        _ => {
            msg!("❌ Invalid instruction type");
            return Err(ChannelError::InvalidInstruction.into());
        }
    };
    msg!("✓ Instruction processed successfully");
    Ok(())
}
