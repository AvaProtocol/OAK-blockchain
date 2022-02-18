use crate::chain_spec::validate_endowment;
use neumann_runtime::{AccountId, Balance};

#[test]
fn validate_neumann_allocation() {
    let allocation_json = &include_bytes!("../../distribution/neumann_alloc.json")[..];
	let allocation: Vec<(AccountId, Balance)> = serde_json::from_slice(allocation_json).unwrap();

    validate_endowment(allocation);
}