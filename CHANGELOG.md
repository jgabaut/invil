# Changelog

## [Unreleased]

### Added

- Support for testmode
  - Add tables for test entries
  - Handle Testmode to run executables
  - Handle -b to record
- Support for test macro
  - Handle running and recording all tests

## [0.0.9] - 2023-12-07

### Added

- Support for -G flag
  - Detailed build info is defined but empty.
  - Also includes build OS into header.

- New logged flag to also output full log to file
  - Output file is ./invil.log

### Changed

- Updated log config to not always show time

## [0.0.8] - 2023-12-06

### Added

- Support for automake mode
  - Naive command to prep autotools

### Changed

- Permit run op for Gitmode

## [0.0.7] - 2023-12-06

### Added

- Support for make builds in Basemode
- Support for Gitmode
  - Still not handling autotools

### Changed

- Updated try_anvil_auto to use the proper version numbers for this repo for tests
- Added support_automakemode to AmbosoEnv

## [0.0.6] - 2023-12-03

### Added

- Add do_run(), to try running the passed tag's binary
- Add do_delete(), to try deleting the passed tag's binary
  - At the moment, only Basemode is supported
- Add functionality for -d, -p

### Changed

- Updated changelog to 0.0.5
- Minor verbosity changes

## [0.0.5] - 2023-12-02

### Added

- Add do_build(), to try building the passed tag
  - At the moment only the basic gcc mode is implemented
- Add functionality for -q
- Add functionality for -i, -b
  - Since both use do_build(), note that make support is still missing

### Fixed

- Refuse -t, -T when env does not support testmode

### Changed

- Version tables in AmbosoEnv changed to BTreeMap

## [0.0.4] - 2023-11-30

### Added

- Add stub op checks for build, run, delete, init, purge
- Add do_query(), checking if requested tag is in supported versions
- Add functionality for -l and -L
- Add functionality for -V

### Fixed

- Fix panic on unwrapping AmbosoEnv.run_mode

### Changed

- handle_amboso_env() takes Args

## [0.0.3] - 2023-11-30

### Added

- AmbosoMode enum definition
- Basic logic to set anvil_env/args value
- Stub handle_amboso_env()

### Fixed

- Added changelog to repo

### Changed

- Extended AmbosoEnv, now includes runmode, selected ops, testmode and makemode support for the run
- Moved some output to trace level
- main() returns ExitCode
- check_passed_args() returns Result<AmbosoEnv,String>

## [0.0.2] - 2023-11-29

### Added

- AmbosoEnv struct definition
- Basic logic to parse amboso_dir/stego.lock as toml
- is_git_repo_clean() to check if working dir status when in git mode

### Fixed

- Set args.git to true when no other runmode is specified

### Changed

- Return early on calls with warranty flag
- Upgrade dependencies: clap 4.4.10

## [0.0.1] - 2023-11-28

### Added

- Basic argument parsing that mostly complies with the bash implementation
- Default for amboso directory ("./bin")
