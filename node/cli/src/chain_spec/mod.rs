// This file is part of OAK.

// Copyright (C) 2018-2021 OAK Network
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

pub mod oak_testnet;

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::Properties;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_core::{Pair, Public};
use sp_runtime::traits::IdentifyAccount;
#[cfg(feature = "std")]
use sp_std::collections::btree_map::BTreeMap;

// TODO(irsal): rm hard-code
use crate::chain_spec::oak_testnet::AccountId;
use crate::chain_spec::oak_testnet::AccountPublic;
pub const OAK_TESTNET_TOKEN: &str = "OAK";

pub const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// set default ss58 crypto
pub fn set_default_ss58_version(spec: &Box<dyn sc_service::ChainSpec>) {
    use sp_core::crypto::Ss58AddressFormat;

    let ss58_version = Ss58AddressFormat::SubstrateAccount;

    sp_core::crypto::set_default_ss58_version(ss58_version);
}

/// Generate chain properties for network.
pub(crate) fn as_properties() -> Properties {
    let (symbol, decimal) = (OAK_TESTNET_TOKEN, 10); // TODO(irsal): Dedupe
    json!({
        "ss58Format": 8, // TODO(irsal): remove hard-code
        "tokenSymbol": symbol,
        "tokenDecimals": decimal,
    })
    .as_object()
    .expect("Network properties are valid; qed")
    .to_owned()
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}