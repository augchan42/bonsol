use anyhow::Result;
use bytes::{Bytes, BytesMut};
use futures_util::{Stream, StreamExt};
use std::time::{SystemTime, Duration};
use std::sync::Arc;
use tracing::{info, error};
use reqwest::Url;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use bonsol_schema::ProgramInputType;
use crate::input_resolver::ResolvedInput;

pub async fn get_body_max_size(
    stream: impl Stream<Item = reqwest::Result<Bytes>> + 'static,
    max_size: usize,
) -> Result<Bytes> {
    let mut max = 0;
    let mut b = BytesMut::new();
    let mut stream = Box::pin(stream);
    while let Some(chunk) = stream.as_mut().next().await {
        let chunk_res = chunk?;
        let chunk = BytesMut::from(chunk_res.as_ref());
        let l = chunk.len();
        max += l;
        if max > max_size {
            return Err(anyhow::anyhow!("Max size exceeded"));
        }
        b.extend_from_slice(&chunk);
    }
    Ok(b.into())
}

pub async fn download_public_input(
    client: Arc<reqwest::Client>,
    index: u8,
    url: Url,
    max_size_mb: usize,
    input_type: ProgramInputType,
    timeout: Duration,
) -> Result<ResolvedInput> {
    info!("Starting download for input {} from {}", index, url);
    let start = SystemTime::now();
    
    let response = match tokio::time::timeout(timeout, client.get(url.clone()).send()).await {
        Ok(Ok(r)) => {
            info!("Received response for input {} after {:?}", 
                index, 
                SystemTime::now().duration_since(start).unwrap_or_default()
            );
            r
        },
        Ok(Err(e)) => {
            error!("HTTP request failed for input {}: {}", index, e);
            error!("URL: {}", url);
            return Err(anyhow::anyhow!("HTTP request failed: {}", e));
        },
        Err(_) => {
            error!("Request timed out for input {} after {:?}", 
                index,
                SystemTime::now().duration_since(start).unwrap_or_default()
            );
            return Err(anyhow::anyhow!("Request timed out"));
        }
    };

    let status = response.status();
    if !status.is_success() {
        error!("HTTP request failed for input {} with status {}", index, status);
        error!("URL: {}", url);
        return Err(anyhow::anyhow!("HTTP request failed with status: {}", status));
    }

    let content_length = response.content_length();
    info!("Content length for input {}: {:?} bytes", index, content_length);

    let max_size = max_size_mb * 1024 * 1024;
    if let Some(len) = content_length {
        if len > max_size as u64 {
            error!("Content length {} exceeds maximum size {} for input {}", 
                len, max_size, index);
            return Err(anyhow::anyhow!("Content too large"));
        }
    }

    let bytes = match tokio::time::timeout(timeout, response.bytes()).await {
        Ok(Ok(b)) => {
            info!("Downloaded {} bytes for input {} in {:?}",
                b.len(),
                index,
                SystemTime::now().duration_since(start).unwrap_or_default()
            );
            b
        },
        Ok(Err(e)) => {
            error!("Failed to read response body for input {}: {}", index, e);
            return Err(anyhow::anyhow!("Failed to read response body: {}", e));
        },
        Err(_) => {
            error!("Body download timed out for input {} after {:?}",
                index,
                SystemTime::now().duration_since(start).unwrap_or_default()
            );
            return Err(anyhow::anyhow!("Body download timed out"));
        }
    };

    if bytes.len() > max_size {
        error!("Downloaded size {} exceeds maximum size {} for input {}", 
            bytes.len(), max_size, index);
        return Err(anyhow::anyhow!("Downloaded content too large"));
    }

    info!("Successfully completed download for input {} ({} bytes) in {:?}",
        index,
        bytes.len(),
        SystemTime::now().duration_since(start).unwrap_or_default()
    );

    Ok(ResolvedInput {
        index,
        data: bytes.to_vec(),
        input_type,
    })
}

pub async fn download_public_account(
    rpc_client: Arc<RpcClient>,
    index: u8,
    pubkey: Pubkey,
    max_size_mb: usize,
) -> Result<ResolvedInput> {
    info!("Starting account data download for input {} ({})", index, pubkey);
    let start = SystemTime::now();

    let account = match rpc_client.get_account(&pubkey).await {
        Ok(a) => {
            info!("Retrieved account data for input {} after {:?}",
                index,
                SystemTime::now().duration_since(start).unwrap_or_default()
            );
            a
        },
        Err(e) => {
            error!("Failed to get account data for input {} ({}): {}", 
                index, pubkey, e);
            return Err(anyhow::anyhow!("Failed to get account data: {}", e));
        }
    };

    let max_size = max_size_mb * 1024 * 1024;
    if account.data.len() > max_size {
        error!("Account data size {} exceeds maximum size {} for input {}", 
            account.data.len(), max_size, index);
        return Err(anyhow::anyhow!("Account data too large"));
    }

    info!("Successfully downloaded account data for input {} ({} bytes) in {:?}",
        index,
        account.data.len(),
        SystemTime::now().duration_since(start).unwrap_or_default()
    );

    Ok(ResolvedInput {
        index,
        data: account.data,
        input_type: ProgramInputType::Public,
    })
}
