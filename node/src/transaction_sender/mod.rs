use std::sync::Arc;

use {
    async_trait::async_trait,
    bonsol_interface::{
        bonsol_schema::{
            ChannelInstruction, ChannelInstructionArgs, ChannelInstructionIxType, ClaimV1,
            ClaimV1Args, StatusTypes, StatusV1, StatusV1Args,
        },
        util::{deployment_address, execution_address, execution_claim_address},
    },
    dashmap::DashMap,
    flatbuffers::FlatBufferBuilder,
    itertools::Itertools,
    solana_rpc_client_api::{
        client_error::Error,
        config::RpcSendTransactionConfig,
    },
    solana_sdk::{
        account::Account,
        commitment_config::CommitmentConfig,
        instruction::{AccountMeta, Instruction, InstructionError},
        message::{v0, VersionedMessage},
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::{Signer, SignerError},
        system_program,
        transaction::{TransactionError, VersionedTransaction},
        compute_budget::ComputeBudgetInstruction,
        system_instruction,
    },
    solana_transaction_status::TransactionStatus as TransactionConfirmationStatus,
    tokio::task::JoinHandle,
    tracing::{error, info, debug},
};

use {
    crate::types::ProgramExec,
    anyhow::Result,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending { expiry: u64 },
    Confirmed(TransactionConfirmationStatus),
}

#[async_trait]
pub trait TransactionSender {
    fn start(&mut self);
    async fn claim(
        &self,
        execution_id: &str,
        requester: Pubkey,
        execution_account: Pubkey,
        block_commitment: u64,
    ) -> Result<Signature>;
    async fn submit_proof(
        &self,
        execution_id: &str,
        requester_account: Pubkey,
        callback_exec: Option<ProgramExec>,
        proof: &[u8],
        execution_digest: &[u8],
        input_digest: &[u8],
        assumption_digest: &[u8],
        committed_outputs: &[u8],
        additional_accounts: Vec<AccountMeta>,
        exit_code_system: u32,
        exit_code_user: u32,
    ) -> Result<Signature>;
    async fn get_current_block(&self) -> Result<u64>;
    fn get_signature_status(&self, sig: &Signature) -> Option<TransactionStatus>;
    fn clear_signature_status(&self, sig: &Signature);
    async fn get_deployment_account(&self, image_id: &str) -> Result<Account>;
}

pub struct RpcTransactionSender {
    pub rpc_client: Arc<RpcClient>,
    pub bonsol_program: Pubkey,
    pub signer: Keypair,
    pub txn_status_handle: Option<JoinHandle<()>>,
    pub sigs: Arc<DashMap<Signature, TransactionStatus>>,
}

impl Signer for RpcTransactionSender {
    fn pubkey(&self) -> Pubkey {
        self.signer.pubkey()
    }

    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.signer.pubkey())
    }

    fn sign_message(&self, message: &[u8]) -> Signature {
        self.signer.sign_message(message)
    }

    fn try_sign_message(
        &self,
        message: &[u8],
    ) -> std::result::Result<Signature, solana_sdk::signer::SignerError> {
        self.signer.try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        false
    }
}

impl RpcTransactionSender {
    pub fn new(rpc_url: String, bonsol_program: Pubkey, signer: Keypair) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            signer,
            bonsol_program,
            txn_status_handle: None,
            sigs: Arc::new(DashMap::new()),
        }
    }

    async fn create_compute_budget_instructions(&self) -> Result<Vec<Instruction>> {
        // Set high compute limit for proof verification
        let compute_limit = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);
        
        // Get current network fees
        let prioritization_fees = self.rpc_client
            .get_recent_prioritization_fees(&[self.signer.pubkey(), self.bonsol_program])
            .await?;
            
        let compute_price = if prioritization_fees.is_empty() {
            ComputeBudgetInstruction::set_compute_unit_price(5)
        } else {
            ComputeBudgetInstruction::set_compute_unit_price(prioritization_fees[0].prioritization_fee)
        };
        
        Ok(vec![compute_limit, compute_price])
    }

    async fn get_rent_exempt_balance(&self, data_len: usize) -> Result<u64> {
        let rent = self.rpc_client.get_minimum_balance_for_rent_exemption(data_len).await?;
        Ok(rent)
    }

    async fn create_rent_funding_instructions(
        &self,
        accounts: &[AccountMeta],
        data_lengths: &[usize],
    ) -> Result<Vec<Instruction>> {
        let mut instructions = Vec::new();
        
        for (i, account) in accounts.iter().enumerate() {
            if !account.is_writable {
                continue;
            }
            
            let account_info = self.rpc_client.get_account(&account.pubkey).await?;
            let required_balance = self.get_rent_exempt_balance(data_lengths.get(i).copied().unwrap_or(0)).await?;
            
            if account_info.lamports < required_balance {
                let transfer_amount = required_balance.saturating_sub(account_info.lamports);
                instructions.push(
                    system_instruction::transfer(
                        &self.signer.pubkey(),
                        &account.pubkey,
                        transfer_amount,
                    )
                );
            }
        }
        
        Ok(instructions)
    }
}

#[async_trait]
impl TransactionSender for RpcTransactionSender {
    fn get_signature_status(&self, sig: &Signature) -> Option<TransactionStatus> {
        self.sigs.get(sig).map(|status| status.value().to_owned())
    }

    fn clear_signature_status(&self, sig: &Signature) {
        self.sigs.remove(sig);
    }

    async fn claim(
        &self,
        execution_id: &str,
        requester: Pubkey,
        execution_account: Pubkey,
        block_commitment: u64,
    ) -> Result<Signature> {
        let (execution_claim_account, _) = execution_claim_address(execution_account.as_ref());
        let accounts = vec![
            AccountMeta::new(execution_account, false),
            AccountMeta::new_readonly(requester, false),
            AccountMeta::new(execution_claim_account, false),
            AccountMeta::new(self.signer.pubkey(), true),
            AccountMeta::new(self.signer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];
        let mut fbb = FlatBufferBuilder::new();
        let eid = fbb.create_string(execution_id);
        let stat = ClaimV1::create(
            &mut fbb,
            &ClaimV1Args {
                block_commitment,
                execution_id: Some(eid),
            },
        );
        fbb.finish(stat, None);
        let statbytes = fbb.finished_data();
        let mut fbb2 = FlatBufferBuilder::new();
        let off = fbb2.create_vector(statbytes);
        let root = ChannelInstruction::create(
            &mut fbb2,
            &ChannelInstructionArgs {
                ix_type: ChannelInstructionIxType::ClaimV1,
                claim_v1: Some(off),
                ..Default::default()
            },
        );
        fbb2.finish(root, None);
        let ix_data = fbb2.finished_data();
        let instruction = Instruction::new_with_bytes(self.bonsol_program, ix_data, accounts);
        
        // Add compute budget instructions
        let mut instructions = self.create_compute_budget_instructions().await?;
        instructions.push(instruction);

        let (blockhash_req, last_valid) = self
            .rpc_client
            .get_latest_blockhash_with_commitment(self.rpc_client.commitment())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {:?}", e))?;

        let msg =
            v0::Message::try_compile(&self.signer.pubkey(), &instructions, &[], blockhash_req)?;
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&self.signer])?;
        let sig = self
            .rpc_client
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {:?}", e))?;
        self.sigs
            .insert(sig, TransactionStatus::Pending { expiry: last_valid });
        Ok(sig)
    }

    async fn submit_proof(
        &self,
        execution_id: &str,
        requester_account: Pubkey,
        callback_exec: Option<ProgramExec>,
        proof: &[u8],
        execution_digest: &[u8],
        input_digest: &[u8],
        assumption_digest: &[u8],
        committed_outputs: &[u8],
        additional_accounts: Vec<AccountMeta>,
        exit_code_system: u32,
        exit_code_user: u32,
    ) -> Result<Signature> {
        let (execution_request_data_account, _) =
            execution_address(&requester_account, execution_id.as_bytes());
        
        info!("🔍 Transaction Construction:");
        info!("Requester Account: {}", requester_account);
        info!("Execution Request Account: {}", execution_request_data_account);
        if let Some(ref pe) = callback_exec {
            info!("Callback Program ID: {}", pe.program_id);
        }
        
        // Handle callback accounts first
        let (program_id, mut accounts) = if let Some(ref pe) = callback_exec {
            let prog = pe.program_id;
            info!("Using callback program: {}", prog);
            //todo: add read interface simulation on program to get other accounts
            (prog, additional_accounts.clone())
        } else {
            info!("No callback program specified");
            (self.bonsol_program, vec![])
        };

        info!("\n📋 Initial Additional Accounts:");
        for (i, acc) in additional_accounts.iter().enumerate() {
            info!("Account {}: {}", i, acc.pubkey);
            info!("  Is Signer: {}", acc.is_signer);
            info!("  Is Writable: {}", acc.is_writable);
        }

        // Prepend the standard accounts in the correct order for the Bonsol program
        info!("\n📋 Prepending Standard Accounts:");
        let standard_accounts = vec![
            AccountMeta::new(requester_account, true),                    // Requester account (signer + writable)
            AccountMeta::new(execution_request_data_account, false),      // Execution request account
            AccountMeta::new_readonly(program_id, false),                 // Callback program
            AccountMeta::new(self.signer.pubkey(), true),                // Prover account (signer)
        ];
        
        for (i, acc) in standard_accounts.iter().enumerate() {
            info!("Standard Account {}: {}", i, acc.pubkey);
            info!("  Is Signer: {}", acc.is_signer);
            info!("  Is Writable: {}", acc.is_writable);
        }
        
        // Insert standard accounts at the beginning
        accounts.splice(0..0, standard_accounts);

        info!("\n📋 Final Account List:");
        for (i, acc) in accounts.iter().enumerate() {
            info!("Account {}: {}", i, acc.pubkey);
            info!("  Is Signer: {}", acc.is_signer);
            info!("  Is Writable: {}", acc.is_writable);
        }

        // Create the main instruction data
        info!("\n🔧 Building Instruction Data:");
        info!("Execution ID: {}", execution_id);
        info!("Proof length: {} bytes", proof.len());
        info!("Execution digest length: {} bytes", execution_digest.len());
        info!("Input digest length: {} bytes", input_digest.len());
        info!("Assumption digest length: {} bytes", assumption_digest.len());
        info!("Committed outputs length: {} bytes", committed_outputs.len());
        info!("Exit codes - System: {}, User: {}", exit_code_system, exit_code_user);
        
        // Create first FlatBuffer for StatusV1
        let mut fbb = FlatBufferBuilder::new();
        
        // 1. Create all vectors first in consistent order
        let proof_vec = fbb.create_vector(proof);
        let execution_digest = fbb.create_vector(execution_digest);
        let input_digest = fbb.create_vector(input_digest);
        let assumption_digest = fbb.create_vector(assumption_digest);
        let eid = fbb.create_string(execution_id);
        let out = fbb.create_vector(committed_outputs);
        
        // 2. Create StatusV1 table with all vectors
        let stat = StatusV1::create(
            &mut fbb,
            &StatusV1Args {
                execution_id: Some(eid),
                status: StatusTypes::Completed,
                proof: Some(proof_vec),
                execution_digest: Some(execution_digest),
                input_digest: Some(input_digest),
                assumption_digest: Some(assumption_digest),
                committed_outputs: Some(out),
                exit_code_system,
                exit_code_user,
            },
        );
        
        // 3. Finish StatusV1 as a nested buffer
        fbb.finish(stat, None);
        let statbytes = fbb.finished_data();
        
        info!("Status bytes length: {} bytes", statbytes.len());
        debug!("Status bytes: {:?}", statbytes);
        
        // Create second FlatBuffer for ChannelInstruction
        let mut fbb2 = FlatBufferBuilder::new();
        let off = fbb2.create_vector(statbytes);
        let root = ChannelInstruction::create(
            &mut fbb2,
            &ChannelInstructionArgs {
                ix_type: ChannelInstructionIxType::StatusV1,
                status_v1: Some(off),
                ..Default::default()
            },
        );
        
        // Finish ChannelInstruction as a root type
        fbb2.finish(root, None);
        let ix_data = fbb2.finished_data();

        info!("Final instruction data length: {} bytes", ix_data.len());
        debug!("Instruction data: {:?}", ix_data);

        // Build all instructions
        info!("\n🔧 Building Instructions:");
        let mut instructions = self.create_compute_budget_instructions().await?;
        info!("Added {} compute budget instructions", instructions.len());
        
        // Add rent funding instructions for callback accounts if needed
        if callback_exec.is_some() {
            info!("Adding rent funding instructions");
            let rent_instructions = self.create_rent_funding_instructions(&accounts, &[0, 0, 14, 0]).await?;
            info!("Added {} rent funding instructions", rent_instructions.len());
            instructions.extend(rent_instructions);
        }
        
        // Add main instruction
        info!("Adding main Bonsol instruction");
        instructions.push(Instruction::new_with_bytes(self.bonsol_program, ix_data, accounts));
        
        info!("\n📋 Final Transaction Summary:");
        info!("Total instructions: {}", instructions.len());
        for (i, ix) in instructions.iter().enumerate() {
            info!("Instruction {}:", i);
            info!("  Program ID: {}", ix.program_id);
            info!("  Number of accounts: {}", ix.accounts.len());
            info!("  Data length: {} bytes", ix.data.len());
        }

        // Create and send transaction
        info!("\n🚀 Preparing to send transaction");
        let (blockhash_req, last_valid) = self
            .rpc_client
            .get_latest_blockhash_with_commitment(self.rpc_client.commitment())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {:?}", e))?;

        info!("Got blockhash: {}", blockhash_req);
        info!("Last valid block: {}", last_valid);

        // Log detailed instruction data
        for (i, instruction) in instructions.iter().enumerate() {
            info!("\n📝 Instruction {} Details:", i);
            info!("Program ID: {}", instruction.program_id);
            info!("Data length: {} bytes", instruction.data.len());
            info!("Raw data: {:?}", instruction.data);
            info!("\nAccounts:");
            for (j, account) in instruction.accounts.iter().enumerate() {
                info!("  Account {}: {}", j, account.pubkey);
                info!("    Is Signer: {}", account.is_signer);
                info!("    Is Writable: {}", account.is_writable);
            }
        }

        let msg = v0::Message::try_compile(&self.signer.pubkey(), &instructions, &[], blockhash_req)?;
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&self.signer])?;

        info!("\n📋 Final Transaction:");
        info!("Number of instructions: {}", tx.message.instructions().len());
        info!("Fee payer: {}", self.signer.pubkey());
        info!("Recent blockhash: {}", blockhash_req);
        
        let sig = self
            .rpc_client
            .send_and_confirm_transaction_with_spinner_and_config(
                &tx,
                CommitmentConfig::confirmed(),
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {:?}", e))?;
            
        info!("Transaction sent successfully");
        info!("Signature: {}", sig);
        
        self.sigs.insert(sig, TransactionStatus::Pending { expiry: last_valid });
        Ok(sig)
    }

    fn start(&mut self) {
        let sigs_ref = self.sigs.clone();
        let rpc_client = self.rpc_client.clone();
        self.txn_status_handle = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let current_block_height = rpc_client
                    .get_block_height_with_commitment(rpc_client.commitment())
                    .await;

                if let Ok(current_block_height) = current_block_height {
                    sigs_ref.retain(|k, v| {
                        if let TransactionStatus::Pending { expiry } = v {
                            if *expiry < current_block_height {
                                info!("Transaction expired {}", k);
                                return false;
                            }
                        }
                        true
                    });
                    let all_sigs = sigs_ref.iter().map(|x| *x.key()).collect_vec();
                    let statuses = rpc_client.get_signature_statuses(&all_sigs).await;
                    if let Ok(statuses) = statuses {
                        for sig in all_sigs.into_iter().zip(statuses.value.into_iter()) {
                            if let Some(status) = sig.1 {
                                sigs_ref.insert(sig.0, TransactionStatus::Confirmed(status));
                            }
                        }
                    }
                } else {
                    error!("Failed to get block height");
                }
            }
        }));
    }

    async fn get_current_block(&self) -> Result<u64> {
        self.rpc_client
            .get_block_height()
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    async fn get_deployment_account(&self, image_id: &str) -> Result<Account> {
        let (deployment_account, _) = deployment_address(image_id);
        self.rpc_client
            .get_account(&deployment_account)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))
    }
}

fn extract_custom_error(error: &Error) -> Option<u32> {
    if let Error { kind: solana_rpc_client_api::client_error::ErrorKind::TransactionError(
        TransactionError::InstructionError(_, InstructionError::Custom(code))
    ), .. } = error {
        Some(*code)
    } else {
        None
    }
}
