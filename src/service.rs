use std::{collections::HashMap, str::FromStr, thread::sleep, time::Duration};

use anyhow::{Ok, Result};
use bitcoincore_rpc::{
    bitcoin::{self, network, Amount, Network, Txid},
    RpcApi,
};
use bitcoind::{BitcoinD, Conf};
use electrsd::{electrum_client::ElectrumApi, ElectrsD};
use rgb_lib::{
    wallet::{Recipient, RecipientData},
    SecretSeal,
};

use crate::User;

pub struct Service {
    pub network: bitcoin::Network,
    pub bitcoind: BitcoinD,
    pub electrsd: ElectrsD,
    pub users: Vec<User>,
}

impl Service {
    pub fn new(network: bitcoin::Network) -> Result<Self> {
        let mut conf = Conf::default();
        conf.p2p = bitcoind::P2P::Yes;

        let bitcoind = bitcoind::BitcoinD::from_downloaded_with_conf(&conf).unwrap();

        let electrs_path = electrsd::downloaded_exe_path().unwrap();
        let electrsd = electrsd::ElectrsD::new(electrs_path, &bitcoind).unwrap();
        let users = Vec::new();
        return Ok(Self {
            network,
            bitcoind,
            electrsd,
            users,
        });
    }

    pub fn sync_to_chain(&self, txid: Option<Txid>) -> Result<()> {
        let blockheight = self.bitcoind.client.get_block_count()?;

        loop {
            sleep(Duration::from_millis(100));
            let mut synced = true;
            if self
                .electrsd
                .client
                .block_header(blockheight as usize)
                .is_err()
            {
                synced = false;
            }
            if synced {
                break;
            }
        }
        if let Some(transaction) = txid {
            self.electrsd.wait_tx(&transaction);
        }
        Ok(())
    }

    pub fn mine(&self, blocks: u64) -> Result<u64> {
        let miner_addr = self
            .bitcoind
            .client
            .get_new_address(None, None)?
            .require_network(Network::Regtest)?;
        let _blocks = self
            .bitcoind
            .client
            .generate_to_address(blocks, &miner_addr)?;
        Ok(blocks)
    }

    pub fn create_user(&mut self, name: &str) -> Result<String> {
        for user in &self.users {
            if user.name == name {
                return Ok(format!("User {} already exists.", name));
            }
        }
        let user = User::new(name, &self.electrsd)?;
        let fp = user.keys.account_xpub_fingerprint.clone();
        self.users.push(user);
        Ok(format!("Created user {}, with fingerprint {}", fp, name))
    }
}

#[test]
fn test_create_user() -> Result<()> {
    // First initialize a service
    let service = Service::new(Network::Regtest)?;

    let user = User::new("testuser", &service.electrsd)?;
    let _ = user.get_address()?;
    service.mine(120)?;
    service.sync_to_chain(None)?;

    Ok(())
}

#[test]
fn test_blind_receive() -> Result<()> {
    let service = Service::new(Network::Regtest)?;
    let recv = User::new("receiver", &service.electrsd)?;

    let addr = recv.get_address()?;
    service.bitcoind.client.generate_to_address(1, &addr)?;
    service.mine(120)?;

    recv.wallet
        .create_utxos(recv.online.clone(), true, Some(5), None, 1.7)?;
    service.sync_to_chain(None)?;
    let recv_data = recv.wallet.blind_receive(
        None,
        None,
        None,
        vec!["rpc://127.0.0.1:3000/json-rpc".to_string()],
        1,
    )?;

    println!("receive data is {}", recv_data.recipient_id);

    Ok(())
}

#[test]
fn test_send() -> Result<()> {
    let service = Service::new(Network::Regtest)?;
    let sender = User::new("sender", &service.electrsd)?;
    let recv = User::new("receiver", &service.electrsd)?;

    let send_addr = sender.get_address()?;
    let recv_addr = recv.get_address()?;

    service.sync_to_chain(None)?;
    let _txid_0 = service
        .bitcoind
        .client
        .generate_to_address(25, &send_addr)?;
    service.bitcoind.client.generate_to_address(1, &recv_addr)?;

    service.mine(120)?;
    service.sync_to_chain(None)?;

    let created_utxo = recv
        .wallet
        .create_utxos(recv.online.clone(), true, Some(5), None, 13.5)?;
    assert_eq!(5, created_utxo);
    service.sync_to_chain(None)?;
    let recv_data = recv.blind_receive()?;

    println!("receive data is {:?}", recv_data.recipient_id);

    let amount = 1;

    sender
        .wallet
        .create_utxos(sender.online.clone(), true, None, None, 1.6)?;

    let nft = sender.issue_nft(
        "NFT".to_string(),
        "non fungible token".to_string(),
        None,
        1_u8,
        Some("rgb.jpeg".to_string()),
        vec!["rgb.jpeg".to_string()],
    )?;
    println!("issued and nft {:?}", nft.asset_id);

    let assets = sender.wallet.list_assets(vec![])?;
    for asset in assets.uda.iter() {
        println!("{:?}", asset);
    }

    let recipient_map = HashMap::from([(
        nft.asset_id.clone(),
        vec![Recipient {
            recipient_data: RecipientData::BlindedUTXO(SecretSeal::from_str(
                &recv_data.recipient_id,
            )?),
            amount,
            transport_endpoints: vec!["rpc://127.0.0.1:3000/json-rpc".to_string()],
        }],
    )]);
    let txid = sender.send_uda(recipient_map, 2.0)?;
    assert!(!txid.is_empty());

    recv.wallet.refresh(recv.online.clone(), None, vec![])?;
    println!("{:?}", nft);

    let txs = recv.wallet.list_transactions(Some(recv.online.clone()))?;
    println!("tx{:?}", txs);
    service.mine(2)?;
    let transfers = recv.wallet.list_transfers(Some(nft.asset_id))?;
    assert!(!transfers.is_empty());
    Ok(())
}
