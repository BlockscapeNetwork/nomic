//!  Start the peg abci server.

use super::state_machine::{initialize, run};
use super::Action;
use crate::core::primitives::transaction::Transaction;
use failure::bail;
use orga::abci::{ABCIStateMachine, Application};
use orga::Result as OrgaResult;
use orga::{merk::MerkStore, Store};
use std::collections::BTreeMap;
use std::path::Path;

use tendermint_proto::abci::{ RequestInitChain, ResponseInitChain, RequestCheckTx, ResponseCheckTx, 
    RequestDeliverTx, ResponseDeliverTx, RequestBeginBlock, ResponseBeginBlock, RequestEndBlock, 
    ResponseEndBlock, ValidatorUpdate
};
//use merk::Merk;
use orga::merk::merk::Merk;

use tendermint_proto::crypto::public_key::Sum;
use tendermint_proto::crypto::PublicKey;

struct App;

impl Application for App {
    fn init_chain<S: Store>(
        &self,
        mut store: S,
        req: RequestInitChain,
    ) -> OrgaResult<ResponseInitChain> {
        let mut validators = BTreeMap::<Vec<u8>, u64>::new();
        
        for validator in req.validators {
            //let pub_key = validator.get_pub_key().get_data().to_vec();
            // Todo VHX
            let pub_key_help = validator.pub_key.unwrap();
            let pub_key = match pub_key_help.sum {
              Some(Sum::Ed25519(pk))  => { pk }
                _ => vec![]
            };

            let power = validator.power as u64;
            validators.insert(pub_key, power);
        }

        write_validators(&mut store, validators)?;
        initialize(&mut store)?;

        Ok(ResponseInitChain {
            consensus_params: None,
            validators: vec![],
            app_hash: vec![],
        })
    }

    fn check_tx<S: Store>(&self, mut store: S, req: RequestCheckTx) -> OrgaResult<ResponseCheckTx> {
        // let tx = serde_json::from_slice::<Transaction>(req.get_tx());
        // Todo VHX
        let tx = serde_json::from_slice::<Transaction>(&*req.tx);
        let mut validators = read_validators(&mut store);

        match tx {
            Ok(tx) => match run(&mut store, Action::Transaction(tx), &mut validators) {
                Ok(_execution_result) => {
                    // TODO: Don't write validators back to store if they haven't changed
                    write_validators(&mut store, validators)?;
                    // TODO: VHX
                    //let mut res = ResponseCheckTx::new();
                    let mut res = ResponseCheckTx {
                        code: 0,
                        data: vec![],
                        log: "".to_string(),
                        info: "".to_string(),
                        gas_wanted: 0,
                        gas_used: 0,
                        events: vec![],
                        codespace: "".to_string()
                    };
                    // res.set_data(vec![]);
                    res.data = vec![];
                    Ok(res)
                }

                Err(e) => bail!("check tx err: {:?}", e),
            },

            Err(_e) => bail!("error deserializing tx (check_tx)"),
        }
    }

    fn deliver_tx<S: Store>(
        &self,
        mut store: S,
        req: RequestDeliverTx,
    ) -> OrgaResult<ResponseDeliverTx> {
        // let tx = serde_json::from_slice::<Transaction>(req.get_tx());
        // Todo VHX
        let tx = serde_json::from_slice::<Transaction>(&*req.tx);
        let mut validators = read_validators(&mut store);
        match tx {
            Ok(tx) => match run(&mut store, Action::Transaction(tx), &mut validators) {
                Ok(_execution_result) => {
                    write_validators(&mut store, validators)?;
                    // TODO: VHX
                    // let mut res = ResponseDeliverTx::new();
                    let mut res = ResponseDeliverTx {
                        code: 0,
                        data: vec![],
                        log: "".to_string(),
                        info: "".to_string(),
                        gas_wanted: 0,
                        gas_used: 0,
                        events: vec![],
                        codespace: "".to_string()
                    };
                    // TODO: VHX
                    // res.set_data(vec![]);
                    res.data = vec![];
                    Ok(res)
                }

                Err(_e) => bail!("error executing tx (deliver_tx)"),
            },
            Err(_e) => bail!("error deserializing tx (deliver_tx)"),
        }
    }

    fn begin_block<S: Store>(
        &self,
        mut store: S,
        req: RequestBeginBlock,
    ) -> OrgaResult<ResponseBeginBlock> {
        // TODO: VHX
        // let header = req.get_header().clone();
        let header = req.header.clone().unwrap();
        let action = Action::BeginBlock(header);
        let mut validators = read_validators(&mut store);
        run(&mut store, action, &mut validators)?;
        write_validators(&mut store, validators)?;
        Ok(Default::default())
    }

    fn end_block<S: Store>(&self, store: S, _req: RequestEndBlock) -> OrgaResult<ResponseEndBlock> {
        let validators = read_validators(store);
        let mut validator_updates: Vec<ValidatorUpdate> = Vec::new();
        for (pub_key_bytes, power) in validators {
            // TODO: VHX
            // let mut validator_update = ValidatorUpdate::new();
            // let mut pub_key = validator_update.pub_key;
            // pub_key.set_data(pub_key_bytes);
            // pub_key.set_field_type(String::from("secp256k1"));
            // validator_update.set_pub_key(pub_key);
            // validator_update.set_power(power as i64);
            // validator_updates.push(validator_update);
            let mut validator_update = ValidatorUpdate { 
                // pub_key: Option::from(PublicKey { sum: None }),
                pub_key: Some(PublicKey { sum: Option::from(Sum::Ed25519(pub_key_bytes)) }),
                power: power as i64
            };
            validator_updates.push(validator_update)
        }

        // response.set_validator_updates(validator_updates.into());
        let mut response = ResponseEndBlock {
            validator_updates,
            consensus_param_updates: None,
            events: vec![]
        };

        Ok(response)
    }
}

fn write_validators<S: Store>(mut store: S, validators: BTreeMap<Vec<u8>, u64>) -> OrgaResult<()> {
    let validator_map_bytes =
        bincode::serialize(&validators).expect("Failed to serialize validator map");
    store.put(b"validators".to_vec(), validator_map_bytes)
}
fn read_validators<S: Store>(store: S) -> BTreeMap<Vec<u8>, u64> {
    let validator_map_bytes = store
        .get(b"validators")
        .expect("Failed to read validator map bytes from store")
        .expect("Validator map was not written to store");
    let validators: Result<BTreeMap<Vec<u8>, u64>, bincode::Error> =
        bincode::deserialize(&validator_map_bytes);
    validators.expect("Failed to deserialize validator map")
}

pub fn start<P: AsRef<Path>>(nomic_home: P) {
    let merk_path = nomic_home.as_ref().join("merk.db");
    let mut merk = Merk::open(merk_path).expect("Failed to open Merk database");
    let store = MerkStore::new(&mut merk);
    ABCIStateMachine::new(App, store)
        .listen("127.0.0.1:26658")
        .unwrap();
}
