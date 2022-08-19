# CHANGELOG
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.7.0] - 2022-08-18
- Adding try-runtime to Turing
- Adding Currencies, PolkadotXcm, and XTokens to Turing closed call filter (Valve) 
- Add xcmp-handler pallet to execute xcm, send xcm, and provide xcm fees
- Add pallet-utility
- Schedule XCM through automation-time extrinsic
- Price triggers (Alpha)

## [1.6.0] - 2022-08-10
- Update to substrate 0.9.26
- Added base XCMP-handler pallet
- Updated XCMP-handler pallet to store chain/currency information
- Updated XCMP-handler pallet to create XCM instructions for a Transact with OAK tokens
- RPC to get accountId for XCM Descend + Transact
- RPC to get fees for scheduling an XCM task
- Update auto-compounding to work with new parachain-staking locks
- Removed tight coupling between valve and automation-time pallet

## [1.5.0] - 2022-07-19
- RPC to get execution fee for automation time
- New auto re-staking automation time feature
- RPC to get ideal frequency for re-staking
- Added proxy pallet
- Added multisig pallet

## [283] - 2022-06-23
Note: Runtime-only release
- Update Neumann and Turing governance parameters

## [1.4.0] - 2022-06-01
- Fixed execution fees for automation time
- Update to substrate 0.9.20
- Added Parallel Heiko tokens
- Changed KUSD to AUSD
- Updating fee burn rate to 20%
- Provide on-chain identity registration

## [1.3.0] - 2022-05-17
- Moving Turing xcm configs to their own file.
- Allow foreign tokens to get stored on-chain
- Enable cross chain token transfers
- Lower existential deposit from .1 to .01
- Adding Karura tokens
- Ensure CollatorRegistration before joining candidate pool
- Update to substrate 0.9.19

## [279] - 2022-04-29
- Change scheduled time slots from minutes to hours. Migration used to clear all tasks in existing maps and queues.
- Change max tasks in a given slot to 256 per hour.
- Added recurring tasks feature to schedule up to 24 recurring executions for a single task.

## [1.2.8] - 2022-04-05
- Updated to substrate 0.9.18

## [277] - 2022-03-14
Note: Runtime-only release
- Disable treasury burn in runtime
- Split update queue behavior for missed and scheduled queues
- Added Democracy pallet and Technical Committee (#bump-tx-version)
- Added Turing initial allocation & vesting for wallets
- Change vesting from the 5th of the month to the 1st of the month

## [1.2.7] - 2022-03-10
- Removed Quadratic funding pallet; can now be found [here](https://github.com/OAK-Foundation/quadratic-funding-pallet)
- Added inclusion fees for OAK transactions
- Modified execution fees
- Added weights for pallet-valve
- Upgraded polkadot client to v0.9.17
- Added vesting on initialize
- Benchmarks for automation time pallet
- Safe math calculations

## [1.2.6] - 2022-03-01
- Add Turing runtime and split chain-specific code (#chain-fork)
- Add telemetry for node infra
- Default Turing chain_spec on genesis has all gates closed
- Adding pallet-vesting for OAK's own vesting schedule and distribution

## [1.2.5] - 2022-02-20
- Automation Time RPC called automationTime_generateTaskId
- Added onfinality neumann cli script
- Missed tasks for a given time window will not be run
- Action: wallet to wallet transfer
- Neumann tasks cannot be scheduled farther than 1 week from today
- Change ss58 prefix to 51 for NEU, TUR, OAK (#chain-fork)

## [1.2.4] - 2022-02-08
### Added
- automationTime pallet deployed for a future events (N=1)
- parts of the valve pallet implemented and deployed
