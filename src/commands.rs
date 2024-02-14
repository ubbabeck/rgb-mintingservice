use clap::Parser;

#[derive(Parser, Debug, Clone, PartialEq)]
#[clap(rename_all = "snake")]
pub(crate) enum Commands {
    Mine {
        blocks: u64,
    },
    CreateUser {
        name: String,
    },
    ReceiveBlind {
        user: String,
    },
    SendBtc {
        amount_sat: u64,
        address: String,
    },
    Sync {
        txid: Option<String>,
    },
    GetAddr {
        user: String,
    },
    SendNft {
        user: String,
        asset_id: String,
        recipient_id: String,
        amount: u64,
        fee: f32,
    },
    IssueNft {
        user: String,
        ticker: String,
        precision: u8,
        name: String,
        attachments_file_paths: String,
    },
    GetBalance {
        user: String,
    },
    MineToAddress {
        address: String,
        num_blocks: u64,
    },
    CreateUtxo {
        user: String,
        fee: f32,
    },
    ListNfts {
        user: String,
    },
    ListTransfers{
        user:String,
    }
}
