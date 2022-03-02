# CHANGELOG
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
- Removed Quadratic funding pallet; can now be found [here](https://github.com/OAK-Foundation/quadratic-funding-pallet)
- Added inclusion fees for OAK transactions

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
