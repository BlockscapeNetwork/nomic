use crate::core::primitives::transaction::Transaction;
// TODO: Should Tendermint messages be wrapped by orga in general?
use tendermint_proto::types::Header;


#[derive(Clone, Debug)]
pub enum Action {
    BeginBlock(Header),
    Transaction(Transaction),
}
