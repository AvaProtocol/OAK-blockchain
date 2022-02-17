# CHANGELOG
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
- Automation Time RPC called automationTime_generateTaskId
- Added onfinality neumann cli script
- Missed tasks for a given time window will not be run
- Action: wallet to wallet transfer
- Neumann tasks cannot be scheduled farther than 1 week from today
- Change ss58 prefix to 51 for NEU, TUR, OAK

## [1.2.4] - 2022-02-08
### Added
- automationTime pallet deployed for a future events (N=1)
- parts of the valve pallet implemented and deployed