mod command_handlers;
mod commands;
mod service;

use std::{collections::HashMap, fs, str::FromStr};

use bitcoincore_rpc::bitcoin::Network;
use clap::Parser;
use core::result::Result::Ok;

use anyhow::{anyhow, ensure, Error, Result};
use bitcoind::{BitcoinD, Conf};
use electrsd::ElectrsD;
use rgb_lib::{
    keys::Keys,
    wallet::{self, AssetUDA, Online, ReceiveData, Recipient, WalletData},
    Wallet,
};
use rustyline::{error::ReadlineError, Editor};

use crate::{command_handlers::handle_command, commands::Commands, service::Service};

fn main() -> Result<()> {
    let mut transport_endpoints = Vec::new();
    transport_endpoints.push(String::from("rpc://127.0.0.1:3000/json-rpc"));
    //let nft_definition = std::env::args().nth(1).expect("No nft definition given");
    //let blinded_utxo =  std::env::args().nth(2).expect("No blided utxo given");

    // First we need to create the enviorment in order to mimic the blockchain.

    let service = &mut Service::new(Network::Regtest)?;

    let mut rl = &mut rustyline::DefaultEditor::new()?;
    print!(
        "
        help to list commands.
        Exit CTRL+C
        "
    );
    loop {
        let readline = rl.readline("rgb-mintingservice > ");

        match readline {
            Ok(line) => {
                let mut vec: Vec<&str> = line.as_str().split_whitespace().collect();
                vec.insert(0, " ");
                let cli_res = Commands::try_parse_from(vec);
                if cli_res.is_err() {
                    println!("{}", cli_res.unwrap_err());
                    continue;
                }
                let res = handle_command(rl, cli_res.unwrap(), service);
                show_results(res);
                continue;
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL+C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("Closing");
                break;
            }
            Err(err) => {
                println!("Closing");
                break;
            }
        }
    }

    Ok(())
}

fn show_results(res: Result<String>) {
    match res {
        Ok(inner) => println!("{inner}"),
        Err(err) => eprintln!("Error: {err}"),
    }
}

#[derive(Parser, Debug)]
pub struct Cli {
    name: String,
    path: std::path::PathBuf,
}

pub struct User {
    name: String,
    keys: Keys,
    wallet: Wallet,
    online: Online,
}

impl User {
    fn new(name: &str, electrsd: &ElectrsD) -> Result<Self> {
        let tmp_dir = format!("data/{}", name).to_string();
        let vanilla_keychain = Some(1);
        fs::create_dir_all(&tmp_dir).ok();
        let keys = rgb_lib::generate_keys(rgb_lib::BitcoinNetwork::Regtest);
        let wallet_data = WalletData {
            data_dir: tmp_dir,
            bitcoin_network: rgb_lib::BitcoinNetwork::Regtest,
            database_type: wallet::DatabaseType::Sqlite,
            max_allocations_per_utxo: 1,
            pubkey: keys.account_xpub.clone(),
            mnemonic: Some(keys.mnemonic.clone()),
            vanilla_keychain: vanilla_keychain,
        };
        let mut wallet = Wallet::new(wallet_data)?;

        let online = wallet.go_online(false, electrsd.electrum_url.clone())?;

        Ok(Self {
            name: name.to_string(),
            keys: keys,
            wallet: wallet,
            online: online,
        })
    }

    fn get_address(&self) -> Result<bitcoincore_rpc::bitcoin::Address> {
        let send_address = self.wallet.get_address().unwrap();
        let send_address = &bitcoincore_rpc::bitcoin::Address::from_str(&send_address)
            .unwrap()
            .require_network(Network::Regtest)?;
        Ok(send_address.clone())
    }

    fn send_uda(
        self,
        receipient: HashMap<String, Vec<Recipient>>,
        fee_rate: f32,
    ) -> Result<String> {
        let result = self
            .wallet
            .send(self.online, receipient.clone(), false, fee_rate, 1)?;
        Ok(result)
    }

    pub fn issue_nft(
        &self,
        ticker: String,
        name: String,
        details: Option<String>,
        precision: u8,
        media_file_path: Option<String>,
        attachments_file_paths: Vec<String>,
    ) -> Result<AssetUDA> {
        Ok(self.wallet.issue_asset_uda(
            self.online.clone(),
            ticker,
            name,
            details,
            precision,
            media_file_path,
            attachments_file_paths,
        )?)
    }
    pub fn blind_receive(&self) -> Result<ReceiveData> {
        Ok(self.wallet.blind_receive(
            None,
            None,
            None,
            vec!["rpc://127.0.0.1:3000/json-rpc".to_string()],
            1,
        )?)
    }
}
