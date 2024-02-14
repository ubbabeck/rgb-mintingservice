use std::{collections::HashMap, str::FromStr};

use anyhow::{Error, Ok};
use bitcoincore_rpc::{
    bitcoin::{self, Txid},
    RpcApi,
};
use rgb_lib::{
    wallet::{Recipient, RecipientData},
    SecretSeal,
};

use crate::{service::Service, Commands};

pub(crate) fn handle_command(command: Commands, service: &mut Service) -> Result<String, Error> {
    match command {
        Commands::Mine { blocks } => {
            let res = service.mine(blocks)?;
            service.sync_to_chain(None)?;
            return serde_json::to_string_pretty(&res).map_err(|e| e.into());
        }
        Commands::CreateUser { name } => {
            let res = service.create_user(&name)?;
            Ok(res)
        }
        Commands::ReceiveBlind { user } => {
            for u in &service.users {
                if user == u.name {
                    let res = u.blind_receive()?;
                    return serde_json::to_string_pretty(&res).map_err(|e| e.into());
                }
            }
            Ok("User not found".to_string())
        }
        Commands::Sync { txid } => {
            if let Some(transaction) = txid {
                let id = Txid::from_str(&transaction)?;
                service.sync_to_chain(Some(id))?;
            } else {
                service.sync_to_chain(None)?;
            }

            Ok("Success".to_string())
        }
        Commands::SendNft {
            user,
            asset_id,
            recipient_id,
            amount,
            fee,
        } => {
            if let Some(user) = service.users.iter().find(|u| u.name == user) {
                let asset = user.wallet.list_assets(vec![])?;
                if let Some(uda) = asset.uda {
                    let mut recipient_map = HashMap::new();
                    for nft in uda {
                        if nft.asset_id == asset_id {
                            recipient_map = HashMap::from([(
                                nft.asset_id.clone(),
                                vec![Recipient {
                                    recipient_data: RecipientData::BlindedUTXO(
                                        SecretSeal::from_str(&recipient_id)?,
                                    ),
                                    amount,
                                    transport_endpoints: vec![
                                        "rpc://127.0.0.1:3000/json-rpc".to_string()
                                    ],
                                }],
                            )]);
                        }
                    }
                    let txid =
                        user.wallet
                            .send(user.online.clone(), recipient_map, false, fee, 1)?;
                    return serde_json::to_string_pretty(&txid).map_err(|e| e.into());
                }
            }
            Ok("User not found".to_string())
        }

        Commands::SendBtc {
            amount_sat,
            address,
        } => {
            let amount = bitcoin::Amount::from_sat(amount_sat);
            let res = service.bitcoind.client.send_to_address(
                &bitcoin::Address::from_str(&address)?.require_network(service.network)?,
                amount,
                None,
                None,
                None,
                None,
                None,
                None,
            )?;
            return Ok(res.to_string());
        }
        Commands::GetAddr { user } => {
            for u in &service.users {
                if user == u.name {
                    let addr = u.get_address()?.to_string();

                    return Ok(addr);
                }
            }
            Ok("User not found".to_string())
        }
        Commands::IssueNft {
            user,
            ticker,
            name,
            precision,
            attachments_file_paths,
        } => {
            if let Some(user) = service.users.iter().find(|u| u.name == user) {
                let unspent = user.wallet.list_unspents(Some(user.online.clone()), true)?;
                if !unspent.is_empty() {
                    let nft = user.issue_nft(
                        ticker,
                        name.clone(),
                        None,
                        precision,
                        None,
                        vec![attachments_file_paths],
                    )?;
                    return serde_json::to_string_pretty(&nft).map_err(|e| e.into());
                }
            }
            Ok("user not found".to_string())
        }
        Commands::GetBalance { user } => {
            if let Some(user) = service.users.iter().find(|u| user == u.name) {
                let res = user.wallet.list_unspents_vanilla(user.online.clone(), 1)?;
                return serde_json::to_string_pretty(&res).map_err(|e| e.into());
            }
            Ok("user not found".to_string())
        }
        Commands::MineToAddress {
            address,
            num_blocks,
        } => {
            let res = service.bitcoind.client.generate_to_address(
                num_blocks,
                &bitcoin::Address::from_str(&address)?.require_network(service.network)?,
            )?;
            return serde_json::to_string_pretty(&res).map_err(|e| e.into());
        }
        Commands::CreateUtxo { user, fee } => {
            if let Some(user) = service.users.iter().find(|u| user == u.name) {
                let res = user
                    .wallet
                    .create_utxos(user.online.clone(), true, None, None, fee)?;
                return Ok(res.to_string());
            }
            Ok("user not found".to_string())
        }
        Commands::ListNfts { user } => {
            if let Some(user) = service.users.iter().find(|u| u.name == user) {
                let asset = user.wallet.list_assets(vec![])?;
                if let Some(uda) = asset.uda {
                    return serde_json::to_string_pretty(&uda).map_err(|e| e.into());
                }
            }
            Ok("User not found".to_string())
        }
        Commands::ListTransfers { user, asset_id } => {
            if let Some(user) = service.users.iter().find(|u| u.name == user) {
                user.wallet.refresh(user.online.clone(), None, vec![])?;
                let res = user.wallet.list_transfers(Some(asset_id))?;
                return serde_json::to_string_pretty(&res).map_err(|e| e.into());
            }
            Ok("User not found".to_string())
        }
    }
}
