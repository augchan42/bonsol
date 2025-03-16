use std::time::Duration;

use anyhow::Result;

use bytes::Bytes;
use futures_util::TryFutureExt;
use log::{debug, error, info, warn};
use num_traits::FromPrimitive;

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{v0, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;

use tokio::time::Instant;

use bonsol_interface::bonsol_schema::{root_as_deploy_v1, root_as_execution_request_v1};
pub use bonsol_interface::bonsol_schema::{
    ClaimV1T, DeployV1T, ExecutionRequestV1T, ExitCode, InputT, InputType, ProgramInputType,
    StatusTypes,
};
use bonsol_interface::claim_state::ClaimStateHolder;
use bonsol_interface::prover_version::ProverVersion;
pub use bonsol_interface::util::*;
pub use bonsol_interface::{instructions, ID};
use instructions::{CallbackConfig, ExecutionConfig, InputRef};

pub use flatbuffers;

pub struct BonsolClient {
    rpc_client: RpcClient,
}

pub enum ExecutionAccountStatus {
    Completed(ExitCode),
    Pending(ExecutionRequestV1T),
}

impl BonsolClient {
    pub fn new(rpc_url: String) -> Self {
        BonsolClient {
            rpc_client: RpcClient::new(rpc_url),
        }
    }

    pub async fn get_current_slot(&self) -> Result<u64> {
        self.rpc_client
            .get_slot()
            .map_err(|_| anyhow::anyhow!("Failed to get slot"))
            .await
    }

    pub fn with_rpc_client(rpc_client: RpcClient) -> Self {
        BonsolClient { rpc_client }
    }

    pub async fn get_deployment_v1(&self, image_id: &str) -> Result<DeployV1T> {
        let (deployment_account, _) = deployment_address(image_id);
        let account = self
            .rpc_client
            .get_account_with_commitment(&deployment_account, CommitmentConfig::confirmed())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))?
            .value
            .ok_or(anyhow::anyhow!("Invalid deployment account"))?;
        let deployment = root_as_deploy_v1(&account.data)
            .map_err(|_| anyhow::anyhow!("Invalid deployment account"))?;
        Ok(deployment.unpack())
    }

    pub async fn get_execution_request_v1(
        &self,
        requester_pubkey: &Pubkey,
        execution_id: &str,
    ) -> Result<ExecutionAccountStatus> {
        let (er, _) = execution_address(requester_pubkey, execution_id.as_bytes());
        let account = self
            .rpc_client
            .get_account_with_commitment(&er, CommitmentConfig::confirmed())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))?
            .value
            .ok_or(anyhow::anyhow!("Invalid execution request account"))?;
        if account.data.len() == 1 {
            let ec =
                ExitCode::from_u8(account.data[0]).ok_or(anyhow::anyhow!("Invalid exit code"))?;
            return Ok(ExecutionAccountStatus::Completed(ec));
        }
        let er = root_as_execution_request_v1(&account.data)
            .map_err(|_| anyhow::anyhow!("Invalid execution request account"))?;
        Ok(ExecutionAccountStatus::Pending(er.unpack()))
    }

    pub async fn get_claim_state_v1<'a>(
        &self,
        requester_pubkey: &Pubkey,
        execution_id: &str,
    ) -> Result<ClaimStateHolder> {
        let (exad, _) = execution_address(requester_pubkey, execution_id.as_bytes());
        let (eca, _) = execution_claim_address(exad.as_ref());
        let account = self
            .rpc_client
            .get_account_with_commitment(&eca, CommitmentConfig::confirmed())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))?
            .value
            .ok_or(anyhow::anyhow!("Invalid claim account"))?;
        Ok(ClaimStateHolder::new(account.data))
    }

    pub async fn download_program(&self, image_id: &str) -> Result<Bytes> {
        let deployment = self.get_deployment_v1(image_id).await?;
        let url = deployment
            .url
            .ok_or(anyhow::anyhow!("Invalid deployment"))?;
        let resp = reqwest::get(url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download program: {:?}", e))?;
        resp.bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download program: {:?}", e))
    }

    pub async fn get_deployment(&self, image_id: &str) -> Result<Option<Account>> {
        let (deployment_account, _) = deployment_address(image_id);
        let account = self
            .rpc_client
            .get_account_with_commitment(&deployment_account, CommitmentConfig::confirmed())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))?;
        Ok(account.value)
    }

    pub async fn get_fees(&self, signer: &Pubkey) -> Result<u64> {
        debug!("Getting fees for signer: {}", signer);
        let fee_accounts = vec![signer.to_owned(), bonsol_interface::ID];
        debug!("Checking prioritization fees for accounts: {:?}", fee_accounts);
        
        let compute_fees = self
            .rpc_client
            .get_recent_prioritization_fees(&fee_accounts)
            .await?;
            
        let fee = if compute_fees.is_empty() {
            info!("No recent prioritization fees found, using default fee: 5");
            5
        } else {
            info!(
                "Using prioritization fee: {} from {} recent fees",
                compute_fees[0].prioritization_fee,
                compute_fees.len()
            );
            compute_fees[0].prioritization_fee
        };
        
        Ok(fee)
    }

    pub async fn deploy_v1(
        &self,
        signer: &Pubkey,
        image_id: &str,
        image_size: u64,
        program_name: &str,
        url: &str,
        inputs: Vec<ProgramInputType>,
    ) -> Result<Vec<Instruction>> {
        let compute_price_val = self.get_fees(signer).await?;
        let instruction =
            instructions::deploy_v1(signer, image_id, image_size, program_name, url, inputs)?;
        let compute = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
        let compute_price = ComputeBudgetInstruction::set_compute_unit_price(compute_price_val);
        Ok(vec![compute, compute_price, instruction])
    }

    pub async fn execute_v1<'a>(
        &self,
        signer: &Pubkey,
        image_id: &str,
        execution_id: &str,
        inputs: Vec<InputRef<'a>>,
        tip: u64,
        expiration: u64,
        config: ExecutionConfig<'a>,
        callback: Option<CallbackConfig>,
        prover_version: Option<ProverVersion>,
    ) -> Result<Vec<Instruction>> {
        debug!(
            "Preparing execute_v1 transaction: image_id={}, execution_id={}, tip={}, expiry={}",
            image_id,
            execution_id,
            tip,
            expiration
        );
        
        debug!("Getting compute fees...");
        let compute_price_val = self.get_fees(signer).await?;
        info!("Using compute price: {}", compute_price_val);

        let fbs_version_or_none = match prover_version {
            Some(version) => {
                debug!("Using specified prover version");
                let fbs_version = version.try_into().expect("Unknown prover version");
                Some(fbs_version)
            }
            None => {
                debug!("Using default prover version");
                None
            }
        };

        debug!("Building execute instruction...");
        let instruction = instructions::execute_v1(
            signer,
            signer,
            image_id,
            execution_id,
            inputs,
            tip,
            expiration,
            config,
            callback,
            fbs_version_or_none,
        )?;
        
        debug!("Setting compute budget...");
        let compute = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
        let compute_price = ComputeBudgetInstruction::set_compute_unit_price(compute_price_val);
        
        info!(
            "Transaction prepared with compute budget:\n\
             - Compute unit limit: 1,000,000\n\
             - Compute unit price: {}\n\
             - Total max cost: {} lamports",
            compute_price_val,
            compute_price_val * 1_000_000
        );
        
        Ok(vec![compute, compute_price, instruction])
    }

    pub async fn send_txn_standard(
        &self,
        signer: impl Signer,
        instructions: Vec<Instruction>,
    ) -> Result<()> {
        info!("Sending standard transaction with {} instructions", instructions.len());
        self.send_txn(signer, instructions, false, 1, 5).await
    }

    pub async fn send_txn(
        &self,
        signer: impl Signer,
        instructions: Vec<Instruction>,
        skip_preflight: bool,
        retry_timeout: u64,
        retry_count: usize,
    ) -> Result<()> {
        let mut rt = retry_count;
        info!(
            "Sending transaction: skip_preflight={}, retry_timeout={}, retry_count={}",
            skip_preflight,
            retry_timeout,
            retry_count
        );
        
        loop {
            debug!("Getting latest blockhash...");
            let blockhash = self.rpc_client.get_latest_blockhash().await?;
            debug!("Got blockhash: {}", blockhash);
            
            debug!("Compiling transaction message...");
            let message = match v0::Message::try_compile(
                &signer.pubkey(),
                &instructions,
                &[],
                blockhash,
            ) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to compile message: {}", e);
                    return Err(anyhow::anyhow!("Failed to compile message: {}", e));
                }
            };
            
            debug!("Creating versioned transaction...");
            let tx = match VersionedTransaction::try_new(VersionedMessage::V0(message), &[&signer]) {
                Ok(tx) => tx,
                Err(e) => {
                    error!("Failed to create transaction: {}", e);
                    return Err(anyhow::anyhow!("Failed to create transaction: {}", e));
                }
            };
            
            info!("Sending transaction to network...");
            let sig = match self
                .rpc_client
                .send_transaction_with_config(
                    &tx,
                    RpcSendTransactionConfig {
                        skip_preflight,
                        max_retries: Some(0),
                        preflight_commitment: Some(self.rpc_client.commitment().commitment),
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(sig) => {
                    info!("Transaction sent successfully, signature: {}", sig);
                    sig
                }
                Err(e) => {
                    error!("Failed to send transaction: {}", e);
                    if rt > 1 {
                        rt -= 1;
                        info!("Retrying... {} attempts remaining", rt);
                        continue;
                    }
                    return Err(anyhow::anyhow!("Failed to send transaction: {}", e));
                }
            };

            let now = Instant::now();
            let confirm_transaction_initial_timeout = Duration::from_secs(retry_timeout);
            
            info!("Waiting for transaction confirmation...");
            let (_, status) = loop {
                let status = self.rpc_client.get_signature_status(&sig).await?;
                debug!("Transaction status: {:?}", status);
                
                if status.is_none() {
                    let blockhash_not_found = !self
                        .rpc_client
                        .is_blockhash_valid(&blockhash, self.rpc_client.commitment())
                        .await?;
                        
                    if blockhash_not_found {
                        warn!("Blockhash {} no longer valid", blockhash);
                        if now.elapsed() >= confirm_transaction_initial_timeout {
                            error!("Transaction confirmation timed out");
                            break (sig, status);
                        }
                    }
                } else {
                    debug!("Got final transaction status");
                    break (sig, status);
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            };

            match status {
                Some(Ok(())) => {
                    info!("Transaction confirmed successfully");
                    return Ok(());
                }
                Some(Err(e)) => {
                    error!("Transaction failed with error: {:?}", e);
                    return Err(anyhow::anyhow!("Transaction Failure Cannot Recover {:?}", e));
                }
                None => {
                    rt -= 1;
                    if rt == 0 {
                        error!("All retry attempts exhausted");
                        return Err(anyhow::anyhow!("Timeout: Failed to confirm transaction"));
                    }
                    info!("Transaction not confirmed yet, retrying... {} attempts remaining", rt);
                }
            }
        }
    }

    pub async fn wait_for_claim(
        &self,
        requester: Pubkey,
        execution_id: &str,
        timeout: Option<u64>,
    ) -> Result<ClaimStateHolder> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let now = Instant::now();
        let mut end = false;
        loop {
            interval.tick().await;
            if now.elapsed().as_secs() > timeout.unwrap_or(0) {
                end = true;
            }
            if let Ok(claim_state) = self.get_claim_state_v1(&requester, execution_id).await {
                return Ok(claim_state);
            }
            if end {
                return Err(anyhow::anyhow!("Timeout"));
            }
        }
    }

    pub async fn wait_for_proof(
        &self,
        requester: Pubkey,
        execution_id: &str,
        timeout: Option<u64>,
    ) -> Result<ExitCode> {
        let current_block = self.get_current_slot().await?;
        let expiry = current_block + 100;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let now = Instant::now();
        loop {
            interval.tick().await;
            if now.elapsed().as_secs() > timeout.unwrap_or(0) {
                return Err(anyhow::anyhow!("Timeout"));
            }
            let status = self
                .get_execution_request_v1(&requester, execution_id)
                .await;
            match status {
                Ok(ExecutionAccountStatus::Pending(req)) => {
                    if req.max_block_height < expiry {
                        return Err(anyhow::anyhow!("Expired"));
                    }
                }
                Ok(ExecutionAccountStatus::Completed(s)) => {
                    return Ok(s);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// Gets the raw account data for an execution request
    pub async fn get_execution_account_data(
        &self,
        requester_pubkey: &Pubkey,
        execution_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        debug!(
            "Getting execution account data for requester={}, execution_id={}",
            requester_pubkey,
            execution_id
        );
        
        let (er, _) = execution_address(requester_pubkey, execution_id.as_bytes());
        debug!("Derived execution address: {}", er);
        
        let account = self
            .rpc_client
            .get_account_with_commitment(&er, CommitmentConfig::confirmed())
            .await
            .map_err(|e| {
                error!("Failed to get account: {}", e);
                anyhow::anyhow!("Failed to get account: {:?}", e)
            })?
            .value;
            
        if let Some(ref acc) = account {
            debug!("Found account with {} bytes of data", acc.data.len());
        } else {
            warn!("Account not found");
        }
        
        Ok(account.map(|acc| acc.data))
    }
}
