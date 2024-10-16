use super::error::Error;
use crate::{
    solana_utils::{auto_compute_limit_and_price, pack_instructions_into_transactions},
    Result,
};
use anchor_lang::AccountDeserialize;
use futures::{stream, StreamExt, TryFutureExt, TryStreamExt};
use itertools::Itertools;
use solana_client::{
    nonblocking::{rpc_client::RpcClient, tpu_client::TpuClient},
    send_and_confirm_transactions_in_parallel::{
        send_and_confirm_transactions_in_parallel, SendAndConfirmConfig,
    },
    tpu_client::TpuClientConfig,
};
use solana_sdk::{
    account::Account, commitment_config::CommitmentConfig, instruction::Instruction,
    message::Message, pubkey::Pubkey, signature::Keypair, signer::EncodableKey, signer::Signer,
    transaction::Transaction,
};
use std::{marker::Send, path::PathBuf, sync::Arc};

type AccountResult<T> = Result<Option<T>, Error>;
type AccountsResult = Result<Vec<(Pubkey, Option<Account>)>, Error>;

#[derive(Debug, Clone)]
pub struct SolanaClientOpts {
    /// Solana keypair file path
    pub wallet: Option<PathBuf>,

    /// Solana RPC URL
    pub url: String,
}

impl SolanaClientOpts {
    pub fn default_wallet_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        path.push(".config/solana/id.json");
        path
    }

    pub fn load_solana_keypair(&self) -> Result<Keypair> {
        let path = self
            .wallet
            .as_ref()
            .cloned()
            .unwrap_or_else(SolanaClientOpts::default_wallet_path);
        Keypair::read_from_file(path).map_err(|_| anyhow::anyhow!("Failed to read keypair"))
    }

    pub fn ws_url(&self) -> String {
        self.url
            .replace("https", "wss")
            .replace("http", "ws")
            .replace("127.0.0.1:8899", "127.0.0.1:8900")
    }

    pub fn rpc_url(&self) -> String {
        self.url.clone()
    }
}

pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
    payer: Keypair,
    opts: SolanaClientOpts,
}

#[async_trait::async_trait]
pub trait GetAccount {
    async fn account(&self, pubkey: &Pubkey) -> AccountResult<Account>;
    async fn accounts(&self, pubkeys: &[Pubkey]) -> AccountsResult;
}

#[async_trait::async_trait]
pub trait GetAnchorAccount: GetAccount {
    async fn anchor_account<T: AccountDeserialize>(&self, pubkey: &Pubkey) -> AccountResult<T>;
    async fn anchor_accounts<T: AccountDeserialize + Send>(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<(Pubkey, Option<T>)>, Error>;
}

impl SolanaClient {
    pub async fn new(opts: SolanaClientOpts) -> Result<Self> {
        Ok(Self {
            rpc_client: Arc::new(RpcClient::new_with_commitment(
                opts.rpc_url(),
                CommitmentConfig::confirmed(),
            )),
            payer: opts.load_solana_keypair()?,
            opts,
        })
    }

    pub fn get_payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    pub async fn send_instructions(
        &mut self,
        ixs: Vec<Instruction>,
        extra_signers: &[Keypair],
        sequentially: bool,
    ) -> Result<()> {
        let (blockhash, _) = self
            .rpc_client
            .as_ref()
            .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
            .await
            .expect("Failed to get latest blockhash");
        let txs = pack_instructions_into_transactions(vec![ixs], &self.payer);
        let mut with_auto_compute: Vec<Message> = Vec::new();
        let keys: Vec<&dyn Signer> = std::iter::once(&self.payer as &dyn Signer)
            .chain(extra_signers.iter().map(|k| k as &dyn Signer))
            .collect();
        for (tx, _) in &txs {
            // This is just a tx with compute ixs. Skip it
            if tx.len() == 2 {
                continue;
            }

            let computed = auto_compute_limit_and_price(
                &self.rpc_client,
                tx.clone(),
                &keys,
                1.2,
                Some(&self.payer.pubkey()),
                Some(blockhash),
            )
            .await
            .unwrap();
            with_auto_compute.push(Message::new(&computed, Some(&self.payer.pubkey())));
        }

        if with_auto_compute.is_empty() {
            return Ok(());
        }

        if sequentially {
            for message in with_auto_compute {
                let mut tx = Transaction::new_unsigned(message);
                tx.sign(&keys, blockhash);

                self.rpc_client
                    .send_and_confirm_transaction(&tx)
                    .await
                    .map_err(|e| anyhow::anyhow!("Transaction failed: {}", e))?;
            }
        } else {
            let tpu_client = TpuClient::new(
                "helium-config-service-cli",
                self.rpc_client.clone(),
                &self.opts.ws_url(),
                TpuClientConfig::default(),
            )
            .await?;

            let results = send_and_confirm_transactions_in_parallel(
                self.rpc_client.clone(),
                Some(tpu_client),
                &with_auto_compute,
                &keys,
                SendAndConfirmConfig {
                    with_spinner: true,
                    resign_txs_count: Some(5),
                },
            )
            .await?;

            if let Some(err) = results.into_iter().flatten().next() {
                return Err(anyhow::Error::from(err));
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl GetAccount for RpcClient {
    async fn account(&self, pubkey: &Pubkey) -> AccountResult<Account> {
        self.get_account_with_commitment(pubkey, self.commitment())
            .map_ok(|response| response.value)
            .map_err(Error::from)
            .await
    }
    async fn accounts(&self, pubkeys: &[Pubkey]) -> AccountsResult {
        async fn get_accounts(
            client: &RpcClient,
            pubkeys: &[Pubkey],
        ) -> Result<Vec<(Pubkey, Option<Account>)>, Error> {
            let accounts = client.get_multiple_accounts(pubkeys).await?;
            Ok(pubkeys
                .iter()
                .cloned()
                .zip(accounts.into_iter())
                .collect_vec())
        }

        stream::iter(pubkeys.to_vec())
            .chunks(100)
            .map(|key_chunk| async move { get_accounts(self, &key_chunk).await })
            .buffered(5)
            .try_concat()
            .await
    }
}

#[async_trait::async_trait]
impl GetAnchorAccount for RpcClient {
    async fn anchor_account<T: AccountDeserialize>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<T>, Error> {
        self.account(pubkey)
            .and_then(|maybe_account| async move {
                maybe_account
                    .map(|account| {
                        T::try_deserialize(&mut account.data.as_ref()).map_err(Error::from)
                    })
                    .transpose()
            })
            .await
    }

    async fn anchor_accounts<T: AccountDeserialize + Send>(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<(Pubkey, Option<T>)>, Error> {
        self.accounts(pubkeys)
            .await?
            .into_iter()
            .map(|(pubkey, maybe_account)| {
                maybe_account
                    .map(|account| {
                        T::try_deserialize(&mut account.data.as_ref()).map_err(Error::from)
                    })
                    .transpose()
                    .map(|deser_account| (pubkey, deser_account))
            })
            .try_collect()
    }
}

#[async_trait::async_trait]
impl GetAccount for SolanaClient {
    async fn account(&self, pubkey: &Pubkey) -> Result<Option<Account>, Error> {
        self.rpc_client.account(pubkey).await
    }
    async fn accounts(&self, pubkeys: &[Pubkey]) -> Result<Vec<(Pubkey, Option<Account>)>, Error> {
        self.rpc_client.accounts(pubkeys).await
    }
}

#[async_trait::async_trait]
impl GetAnchorAccount for SolanaClient {
    async fn anchor_account<T: AccountDeserialize>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<T>, Error> {
        self.rpc_client.anchor_account(pubkey).await
    }
    async fn anchor_accounts<T: AccountDeserialize + Send>(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<(Pubkey, Option<T>)>, Error> {
        self.rpc_client.anchor_accounts(pubkeys).await
    }
}
