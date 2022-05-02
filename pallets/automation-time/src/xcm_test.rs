#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

use sp_std::prelude::*;
use frame_support::RuntimeDebug;

#[derive(Encode, Decode, RuntimeDebug)]
pub enum SystemCall {
    #[codec(index = 8)]
    RemarkWithEvent(Vec<u8>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum TestChainCall {
    #[codec(index = 0)]
    System(SystemCall),
}

pub struct TestChainCallBuilder;

impl TestChainCallBuilder {
    pub fn remark_with_event(message: Vec<u8>) -> TestChainCall {
        TestChainCall::System(SystemCall::RemarkWithEvent(message))
    }
}
