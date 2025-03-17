use crate::error::ClientError;
use crate::util::execution_address;
use bonsol_schema::root_as_execution_request_v1;
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::program_memory::sol_memcmp;
use solana_program::pubkey::Pubkey;
use tracing::{debug, error, info};

pub struct BonsolCallback<'a> {
    pub input_digest: &'a [u8],
    pub committed_outputs: &'a [u8],
}
/// This is the callback handler for the bonsol program, use this to properly validate an incoming callback from bonsol
/// Ensure you strip the instruction prefix from the data before passing it to this function and that the Execution Id
/// matches the one in the execution request account
pub fn handle_callback<'a>(
    image_id: &str,
    execution_account: &Pubkey,
    accounts: &[AccountInfo],
    stripped_data: &'a [u8],
) -> Result<BonsolCallback<'a>, ProgramError> {
    debug!("\n🔍 Callback Validation Details:");
    debug!("Expected execution account: {}", execution_account);
    debug!("Expected image ID: {}", image_id);
    
    let er_info = accounts
        .first()
        .ok_or::<ProgramError>(ClientError::InvalidCallbackInstructionAccounts.into())?;
    
    debug!("First account details:");
    debug!("  Key: {}", er_info.key);
    debug!("  Owner: {}", er_info.owner);
    debug!("  Is Signer: {}", er_info.is_signer);
    debug!("  Data Length: {}", er_info.data_len());
    
    if sol_memcmp(er_info.key.as_ref(), execution_account.as_ref(), 32) != 0 {
        error!("❌ Execution account mismatch");
        error!("  Expected: {}", execution_account);
        error!("  Got: {}", er_info.key);
        return Err(ClientError::InvalidCallbackInstructionAccounts.into());
    }
    
    if sol_memcmp(er_info.owner.as_ref(), crate::util::ID.as_ref(), 32) != 0 {
        error!("❌ Owner mismatch");
        error!("  Expected: {}", crate::util::ID);
        error!("  Got: {}", er_info.owner);
        return Err(ClientError::InvalidCallbackInstructionAccounts.into());
    }
    
    if !er_info.is_signer {
        error!("❌ Account is not a signer");
        return Err(ClientError::InvalidCallbackSignature.into());
    }
    
    let er_data = &er_info.try_borrow_data()?;
    if er_data.len() < 2 {
        error!("❌ Invalid data length: {}", er_data.len());
        return Err(ClientError::ExecutionRequestReused.into());
    }
    
    // Ensure this is a valid execution request data
    let er = match root_as_execution_request_v1(er_data) {
        Ok(er) => {
            debug!("✓ Successfully parsed execution request data");
            er
        }
        Err(e) => {
            error!("❌ Failed to parse execution request data: {:?}", e);
            return Err(ProgramError::InvalidInstructionData);
        }
    };
    
    if er.image_id() != Some(image_id) {
        error!("❌ Image ID mismatch");
        error!("  Expected: {}", image_id);
        error!("  Got: {:?}", er.image_id());
        return Err(ClientError::InvalidCallbackImageId.into());
    }
    
    info!("✓ All validations passed");
    let (input_digest, committed_outputs) = stripped_data.split_at(32);
    Ok(BonsolCallback {
        input_digest,
        committed_outputs,
    })
}

pub fn handle_callback_id<'a>(
    image_id: &str,
    execution_id: &str,
    request_account: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<BonsolCallback<'a>, ProgramError> {
    let (execution_account, _) = execution_address(request_account, execution_id.as_bytes());
    handle_callback(image_id, &execution_account, accounts, data)
}
