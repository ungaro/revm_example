use alloy_primitives::{Address, Bytes as aBytes};
use anyhow::{Ok, Result};
use ethers_contract::BaseContract;
use ethers_core::abi::parse_abi;
use ethers_providers::{Http, Provider};
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{ExecutionResult, Output, TransactTo, U256 as rU256},
    Database, EVM,
};

use std::{str::FromStr, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let http_url = "http://192.168.1.168:8545";
    let client = Provider::<Http>::try_from(http_url)?;
    let client = Arc::new(client);

    let mut ethersdb = EthersDB::new(client.clone(), None).unwrap();

    // WETH-USDT Uniswap V2 pool
    let pool_address = Address::from_str("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852")?;
    let acc_info = ethersdb.basic(pool_address).unwrap().unwrap();

    let slot = rU256::from(8);
    let value = ethersdb.storage(pool_address, slot).unwrap();
    println!("STORAGE SLOT {:?} {:?}", slot, value); 

    let mut cache_db = CacheDB::new(EmptyDB::default());
    cache_db.insert_account_info(pool_address, acc_info);
    cache_db
        .insert_account_storage(pool_address, slot, value)
        .unwrap();

    let mut evm = EVM::new();
    evm.database(cache_db);

    let pool_contract = BaseContract::from(
        parse_abi(&[
            "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
        ])?
    );

    let encoded = pool_contract.encode("getReserves", ())?;
    evm.env.tx.caller = Address::from_str("0x0000000000000000000000000000000000000000")?;
    evm.env.tx.transact_to = TransactTo::Call(pool_address);
    evm.env.tx.data = aBytes::from(encoded.0);
    evm.env.tx.value = rU256::ZERO;

    let ref_tx = evm.transact_ref().unwrap();
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(value) => Some(value),
            _ => None,
        },
        _ => None,
    };
    println!("Execution Result: {:?}", value);

    let (reserve0, reserve1, ts): (u128, u128, u32) =
        pool_contract.decode_output("getReserves", value.unwrap())?;

    println!(
        "reserve0: {:?} reserve1: {:?} timestamp: {:?}",
        reserve0, reserve1, ts
    );

    Ok(())
}
