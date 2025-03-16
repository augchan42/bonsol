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
        info!("\n🔍 Checking Rent Funding Requirements");
        debug!("Number of accounts to check: {}", accounts.len());
        debug!("Data lengths provided: {:?}", data_lengths);
        
        let mut instructions = Vec::new();
        
        for (i, account) in accounts.iter().enumerate() {
            debug!("\nAccount {} Analysis:", i);
            debug!("Address: {}", account.pubkey);
            debug!("Is Writable: {}", account.is_writable);
            
            if !account.is_writable {
                debug!("Skipping non-writable account");
                continue;
            }
            
            info!("Fetching account info for {}", account.pubkey);
            let account_info = match self.rpc_client.get_account(&account.pubkey).await {
                Ok(info) => {
                    debug!("✓ Account info retrieved");
                    debug!("Current balance: {} lamports", info.lamports);
                    debug!("Current data size: {} bytes", info.data.len());
                    info
                }
                Err(e) => {
                    debug!("Account not found (expected for new accounts): {:?}", e);
                    debug!("Proceeding with zero balance assumption");
                    solana_sdk::account::Account {
                        lamports: 0,
                        data: vec![],
                        owner: solana_sdk::system_program::ID,
                        executable: false,
                        rent_epoch: 0,
                    }
                }
            };
            
            let data_len = data_lengths.get(i).copied().unwrap_or(0);
            debug!("Required data length: {} bytes", data_len);
            
            debug!("Calculating rent-exempt balance...");
            let required_balance = match self.get_rent_exempt_balance(data_len).await {
                Ok(balance) => {
                    debug!("✓ Required balance calculated: {} lamports", balance);
                    balance
                }
                Err(e) => {
                    error!("Failed to calculate rent-exempt balance: {:?}", e);
                    return Err(e.into());
                }
            };
            
            if account_info.lamports < required_balance {
                let transfer_amount = required_balance.saturating_sub(account_info.lamports);
                info!("💰 Account {} requires funding:", account.pubkey);
                debug!("Current balance: {} lamports", account_info.lamports);
                debug!("Required balance: {} lamports", required_balance);
                debug!("Transfer amount: {} lamports", transfer_amount);
                
                debug!("Creating transfer instruction...");
                instructions.push(
                    system_instruction::transfer(
                        &self.signer.pubkey(),
                        &account.pubkey,
                        transfer_amount,
                    )
                );
                debug!("✓ Transfer instruction created");
            } else {
                debug!("✓ Account {} is sufficiently funded", account.pubkey);
                debug!("Current: {} lamports", account_info.lamports);
                debug!("Required: {} lamports", required_balance);
            }
        }
        
        info!("\n📋 Rent Funding Summary: {} instructions created", instructions.len());
        if !instructions.is_empty() {
            debug!("Transfer Instructions:");
            for (i, ix) in instructions.iter().enumerate() {
                debug!("Instruction {}:", i);
                debug!("  From: {}", self.signer.pubkey());
                debug!("  To: {}", ix.accounts[1].pubkey);
                // The amount is the last 8 bytes of the instruction data
                let amount = if ix.data.len() >= 8 {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&ix.data[ix.data.len()-8..]);
                    u64::from_le_bytes(bytes)
                } else {
                    0
                };
                debug!("  Amount: {} lamports", amount);
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
        info!("Step 4/7: Submit Proof [Prover]");
        info!("🔍 Transaction Construction:");
        info!("Requester Account: {}", requester_account);
        info!("Signer Account (Prover): {}", self.signer.pubkey());
        
        let dev_mode = std::env::var("RISC0_DEV_MODE").is_ok();
        if dev_mode {
            info!("⚠️ Running in RISC0_DEV_MODE - using simplified proof verification");
        }
        
        let (execution_request_data_account, _) =
            execution_address(&requester_account, execution_id.as_bytes());
        info!("Status: Checking execution account: {}", execution_request_data_account);
        
        // Verify execution account exists before proceeding
        match self.rpc_client.get_account(&execution_request_data_account).await {
            Ok(_) => info!("✓ Execution account found and verified"),
            Err(e) => {
                error!("❌ Execution account not found or inaccessible");
                error!("Account: {}", execution_request_data_account);
                error!("Error: {:?}", e);
                return Err(anyhow::anyhow!("Execution account not found: {}", e));
            }
        }
        
        // Use original account structure but with enhanced logging
        let (program_id, mut accounts) = if let Some(ref pe) = callback_exec {
            info!("Status: Setting up callback configuration");
            info!("Using callback program: {}", pe.program_id);
            info!("Additional accounts provided: {}", additional_accounts.len());
            for (i, acc) in additional_accounts.iter().enumerate() {
                info!("Additional Account {}: {}", i, acc.pubkey);
                info!("  Is Signer: {}", acc.is_signer);
                info!("  Is Writable: {}", acc.is_writable);
            }
            (pe.program_id, additional_accounts)
        } else {
            info!("Status: No callback program specified");
            (self.bonsol_program, vec![])
        };

        info!("\nStatus: Building standard accounts");
        info!("1. Requester Account: {}", requester_account);
        info!("   Is Signer: true, Is Writable: true");
        info!("2. Execution Account: {}", execution_request_data_account);
        info!("   Is Signer: false, Is Writable: true");
        info!("3. Callback Program: {}", program_id);
        info!("   Is Signer: false, Is Writable: false");
        info!("4. Prover Account: {}", self.signer.pubkey());
        info!("   Is Signer: true, Is Writable: true");

        // Create standard accounts vector with correct permissions
        let mut standard_accounts = vec![
            AccountMeta::new(requester_account, true),
            AccountMeta::new(execution_request_data_account, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new(self.signer.pubkey(), true),
        ];

        info!("Status: Checking account funding requirements");
        // Add extra accounts from callback if present
        if let Some(ref pe) = callback_exec {
            info!("Status: Adding callback extra accounts");
            info!("Instruction prefix: {:?}", pe.instruction_prefix);
            for (i, acc) in accounts.iter().enumerate() {
                info!("Extra Account {}: {}", i, acc.pubkey);
                info!("  Is Signer: {}", acc.is_signer);
                info!("  Is Writable: {}", acc.is_writable);
                
                // Verify each account exists before proceeding
                if acc.is_writable {
                    info!("Status: Checking writable account {}", acc.pubkey);
                    match self.rpc_client.get_account(&acc.pubkey).await {
                        Ok(_) => info!("✓ Account {} exists", acc.pubkey),
                        Err(e) => {
                            error!("❌ Required account not found: {}", acc.pubkey);
                            error!("Error: {:?}", e);
                            return Err(anyhow::anyhow!("Required account not found: {}", e));
                        }
                    }
                }
            }
            standard_accounts.extend(accounts);
        }

        // Log final account configuration
        info!("\n📋 Final Account Configuration:");
        for (i, acc) in standard_accounts.iter().enumerate() {
            info!("Account {}: {}", i, acc.pubkey);
            info!("  Is Signer: {}", acc.is_signer);
            info!("  Is Writable: {}", acc.is_writable);
            if acc.pubkey == system_program::id() {
                info!("  Type: System Program");
            } else if acc.pubkey == program_id {
                info!("  Type: Program ID");
            } else if acc.pubkey == self.signer.pubkey() {
                info!("  Type: Signer");
            } else if acc.pubkey == requester_account {
                info!("  Type: Requester");
            } else if acc.pubkey == execution_request_data_account {
                info!("  Type: Execution Account");
            } else {
                info!("  Type: Additional Account");
            }
        }

        // Build compute budget instructions
        info!("\nStatus: Building instructions");
        let mut instructions = self.create_compute_budget_instructions().await?;
        info!("Added {} compute budget instructions", instructions.len());
        
        // Add rent funding instructions if needed
        if callback_exec.is_some() {
            info!("Status: Adding rent funding instructions");
            let rent_instructions = self.create_rent_funding_instructions(&standard_accounts, &[0, 0, 14, 0]).await?;
            info!("Added {} rent funding instructions", rent_instructions.len());
            instructions.extend(rent_instructions);
        }

        // Create the main instruction data
        info!("\n🔧 Building Instruction Data:");
        let mut fbb = FlatBufferBuilder::new();
        info!("Creating instruction data vectors:");
        info!("  Proof length: {} bytes", proof.len());
        info!("  Execution digest length: {} bytes", execution_digest.len());
        info!("  Input digest length: {} bytes", input_digest.len());
        info!("  Assumption digest length: {} bytes", assumption_digest.len());
        info!("  Committed outputs length: {} bytes", committed_outputs.len());
        info!("  Execution ID: {}", execution_id);
        
        let proof_vec = fbb.create_vector(proof);
        let execution_digest = fbb.create_vector(execution_digest);
        let input_digest = fbb.create_vector(input_digest);
        let assumption_digest = fbb.create_vector(assumption_digest);
        let eid = fbb.create_string(execution_id);
        let out = fbb.create_vector(committed_outputs);
        
        info!("Creating StatusV1 with:");
        info!("  Status: Completed");
        info!("  Exit codes - System: {}, User: {}", exit_code_system, exit_code_user);
        
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
        fbb.finish(stat, None);
        let statbytes = fbb.finished_data();
        
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
        fbb2.finish(root, None);
        let ix_data = fbb2.finished_data();

        // Add main instruction
        instructions.push(Instruction::new_with_bytes(self.bonsol_program, ix_data, standard_accounts));
        
        info!("\n📋 Final Transaction Summary:");
        for (i, ix) in instructions.iter().enumerate() {
            info!("Instruction {}:", i);
            info!("  Program ID: {}", ix.program_id);
            info!("  Number of accounts: {}", ix.accounts.len());
        }

        info!("\nStatus: Preparing to submit proof transaction");
        if dev_mode {
            info!("Dev Mode: Using simplified proof data structures");
        }
        
        let (blockhash, last_valid) = self
            .rpc_client
            .get_latest_blockhash_with_commitment(self.rpc_client.commitment())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {:?}", e))?;

        let msg = v0::Message::try_compile(&self.signer.pubkey(), &instructions, &[], blockhash)?;
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&self.signer])?;

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
            
        info!("Transaction sent successfully: {}", sig);
        
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
