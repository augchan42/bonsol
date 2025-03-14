use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_memory::{sol_memcpy, sol_memset},
    rent::Rent,
    system_instruction,
    msg,
};

use crate::error::ChannelError;
pub fn cleanup_execution_account(
    exec: &AccountInfo,
    requester: &AccountInfo,
    exit_code: u8,
    input_digest: Option<&[u8]>,
) -> Result<(), ProgramError> {
    let size = if let Some(digest) = input_digest {
        exec.realloc(33, false)?;  // 1 byte exit code + 32 bytes input digest
        let mut data = exec.data.borrow_mut();
        data[0] = exit_code;
        data[1..].copy_from_slice(digest);
        33
    } else {
        exec.realloc(1, false)?;
        sol_memset(&mut exec.data.borrow_mut(), exit_code, 1);
        1
    };
    msg!("Cleaned up execution account with {} bytes", size);
    refund(exec, requester)
}

pub fn refund(exec: &AccountInfo, requester: &AccountInfo) -> Result<(), ProgramError> {
    //leave min lamports in the account so that account reuse is not possible
    let lamports = Rent::default().minimum_balance(1);
    let refund = exec.lamports();
    **exec.try_borrow_mut_lamports()? = lamports;
    **requester.try_borrow_mut_lamports()? += refund - lamports;
    Ok(())
}

pub fn payout_tip(exec: &AccountInfo, prover: &AccountInfo, tip: u64) -> Result<(), ProgramError> {
    **exec.try_borrow_mut_lamports()? -= tip;
    **prover.try_borrow_mut_lamports()? += tip;
    Ok(())
}

pub fn transfer_unowned<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    lamports: u64,
) -> Result<(), ProgramError> {
    let ix = system_instruction::transfer(from.key, to.key, lamports);
    invoke(&ix, &[from.clone(), to.clone()])
}

pub fn transfer_owned(
    from: &AccountInfo,
    to: &AccountInfo,
    lamports: u64,
) -> Result<(), ProgramError> {
    **from.try_borrow_mut_lamports()? -= lamports;
    **to.try_borrow_mut_lamports()? += lamports;
    Ok(())
}

pub fn save_structure<'a>(
    account: &'a AccountInfo<'a>,
    seeds: &[&[u8]],
    bytes: &[u8],
    payer: &'a AccountInfo<'a>,
    system: &'a AccountInfo<'a>,
    additional_lamports: Option<u64>,
) -> Result<(), ChannelError> {
    let space = bytes.len() as u64;
    create_program_account(account, seeds, space, payer, system, additional_lamports)?;
    sol_memcpy(&mut account.data.borrow_mut(), bytes, space as usize);
    Ok(())
}

pub fn create_program_account<'a>(
    account: &'a AccountInfo<'a>,
    seeds: &[&[u8]],
    space: u64,
    payer: &'a AccountInfo<'a>,
    system: &'a AccountInfo<'a>,
    additional_lamports: Option<u64>,
) -> Result<(), ChannelError> {
    let lamports =
        Rent::default().minimum_balance(space as usize) + additional_lamports.unwrap_or(0);
    let create_pda_account_ix =
        system_instruction::create_account(payer.key, account.key, lamports, space, &crate::id());
    invoke_signed(
        &create_pda_account_ix,
        &[account.clone(), payer.clone(), system.clone()],
        &[seeds],
    )
    .map_err(|_e| ChannelError::InvalidSystemProgram)
}
