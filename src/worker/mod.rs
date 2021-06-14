use crate::chain::client::Client as PegClient;
use crate::core::work::work;
use blocking::block_on;
use log::info;
use rand::random;
use sha2::{Digest, Sha256};
use tendermint_rpc::Client;
use tokio::runtime::Runtime;

const MIN_WORK: u64 = 1 << 20;

pub fn generate() {
    let rpc = PegClient::new("tcp://127.0.0.1:26657").unwrap();
    // info!("Worker is not implemented correctly!");
    // Borrow issue fixed
    let validators = rpc.tendermint_rpc.clone();
    let mut rt = Runtime::new().unwrap();
    let status = rt.block_on(rpc.tendermint_rpc.status())
        .expect("Unable to connect to tendermint RPC");
    let pub_key_bytes = status.validator_info.pub_key.as_bytes();

    /*
    let pub_key_bytes = rt.block_on(rpc.tendermint_rpc.status())
        .expect("Unable to connect to tendermint RPC")
        .validator_info
        .pub_key
        .as_bytes()
        ;
*/
    let mut nonce = random::<u64>();

    loop {
        let work_value = try_nonce(pub_key_bytes, nonce);

        if work_value >= MIN_WORK {
            info!("Generated {} voting power", work_value);

            rpc.submit_work_proof(&pub_key_bytes.to_vec(), nonce)
                .expect("Failed to submit work proof");
        }

        nonce += 1;
    }

}

fn try_nonce(pub_key_bytes: &[u8], nonce: u64) -> u64 {
    let mut hasher = Sha256::new();
    hasher.input(pub_key_bytes);
    let nonce_bytes = nonce.to_be_bytes();
    hasher.input(&nonce_bytes);
    let hash = hasher.result().to_vec();

    work(&hash)
}
