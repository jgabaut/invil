# Changelog

## [0.2.21] - 2025-03-31

### Changed

- Solve clippy lints
- Bump deps
- Bump expected amboso version to 2.0.10

## [0.2.20] - 2024-12-20

### Changed

- Fixed some wrong Command usages

## [0.2.19] - 2024-11-26

### Added

- Add -Z to pass CFLAGS to single file build mode
  - Avoids reading CFLAGS from env
- Basic optional support for AnvilCustom kern

### Changed

- Fix: avoid error on wrong tag for -G, mimicking amboso
- BaseMode cd is now done through env::set_current_dir()
- Refactored BaseMode do_build() to also use build_step() for queries above anvil_env.makevers
- Fix: anvilPy does not try to call gcc when query >= makevers
- Fix: anvilPy does not try to do autotools prep when query >= automakevers
- Try reading AMBOSO_CONFIG_ARG_ISFILE to use -C with flags directly
  - Setting it to 0 enables the new, backwards incompatible behaviour
- Bump expected amboso version to 2.0.9

## [0.2.18] - 2024-10-24

### Changed

- Fix: avoid error on relative paths with no `./` for `-x`, `-Lx`
- Fix: don't try parsing global conf when file doesn't exist
- Bump expected amboso version to 2.0.8

## [0.2.17] - 2024-08-29

### Added

- Read global config file from $HOME/.anvil/anvil.toml

### Changed

- Fix C header generation not using passed tag's info
- Bump expected amboso version to 2.0.7

## [0.2.16] - 2024-08-10

### Changed

- Print warranty info after version splash
- Bump deps

## [0.2.15] - 2024-07-03

### Changed

- Removed mention of an extension from README, since it was dropped in a previous patch
- Bump deps

## [0.2.14] - 2024-06-21

### Changed

- Ignore untracked files for git check, to match amboso behaviour
- Bump dependencies
- Use dep: syntax to hide implicit features

## [0.2.13] - 2024-04-27

### Fixed

- Test mode actually records test when -b is passed

## [0.2.12] - 2024-04-19

### Fixed

- Allow runs as 2.0.6

## [0.2.11] - 2024-04-19

### Changed

- Handle --strict with init subcommand
- Bump expected amboso version to 2.0.6

## [0.2.10] - 2024-04-13

### Changed

- Moved anvilPy code behind a feature
- Updated init subcommand to use the passed directory basename for the target

## [0.2.9] - 2024-03-26

### Changed

- Fixed generated Makefile.am
- Updated expected amboso version to 2.0.5
- Handle strict behaviour to refuse anvilPy

## [0.2.8] - 2024-02-21

### Added

- New WIP anvil_py module to support python projects
- AmbosoEnv holds AnvilPyEnv

### Changed

- Don't prepend ./ to build_dir in do_build()
- Refactor stego toml parsing into inner function
- Experimental: Accept 2.1.0 from stego.lock

## [0.2.7] - 2024-02-16

### Changed

- Try to create amboso dir  when missing
- Try to create queried dirpath when missing in build op
- Update init command to create stego.lock at target dir
- Bump expected amboso to 2.0.4

## [0.2.6] - 2024-02-09

### Added

- Experimental Makefile parsing, with -Xx
  - Handle -d, -L, -q when in Makefile parser mode for output control
- New -O flag to pass stego_dir path
- New constants to lock in patch-specific behaviour of amboso >2.0
- Interpreter branch for stego.lock in do_query()
- Print compatibility level with -v -V(>3)
- Try parsing legacy format stego.lock when running as <2.0

### Changed

- Expect stego.lock at stego_dir, not amboso_dir
  - New default is "."
  - For now, there's a safety check to still look at the old path when failing to find stego at first
  - Our stego.lock was moved to repo root
- Try reading anvil_kern from stego.lock

## [0.2.5] - 2024-01-22

### Added

- Generated C header contains generation time

## [0.2.4] - 2024-01-10

### Added

- Add -a to set target amboso version
- Add anvil_version to AmbosoEnv
- Read anvil_version from stego.lock and use it to force --strict on 2.0
- Add AnvilKern stub for AmbosoC
- Add -k to set anvil_kern

### Fixed

- Generated configure.ac for init subcommand uses globs to match host
- Working anvil symlink creation for init subcommand

### Changed

- Bump EXPECTED_AMBOSO_API_LEVEL to 2.0.2

## [0.2.3] - 2024-01-04

### Added

- Add -e short flag for strict

### Changed

- Bump expected amboso version to 2.0.1

## [0.2.2] - 2023-12-31

### Added

- Add --strict flag to turn off extensions to amboso 2.0
- print_extension_args() to trace passed flags
- Ignore missing repo in current directory (turn off with --strict)
- Try checking ./stego.lock when missing amboso_dir (turn off with --strict)

### Changed

- Less noisy logs, by not logging intermediate values before the full AmbosoEnv
- Set testmode support to false when missing info from stego.lock

## [0.2.1] - 2023-12-26

### Fixed

- Check later if we're supposed to run make
- Typo for configure.ac path

### Changed

- Rename handle_empty_subcommand() to handle_running_make()

## [0.2.0] - 2023-12-26

### Added

- New short versions of extensions, compatible with amboso 2.0
- Tests for is_semver()
- Read CFLAGS from env for do_build()
- Read CC from env for do_build() when below ANVIL_MAKEVERS
- Try doing make when no args are provided

### Changed

- Test failure returns 1
- is_semver() rejects build and prerelease metadata
- Log file is now anvil.log
- Expected amboso version bumped to 2.0.0

## [0.1.7] - 2023-12-20

### Added

- -Lx functionality
- Tests for semver_compare()
- Added better logic for SemVerKey comparison
- Count test results in test macro

### Changed

- Error if two key conflict when trying to fill version maps
- Internal ordering of version maps
- Test macro returns Err if any test failed

## [0.1.6] - 2023-12-17

### Added

- --force flag to force build
- --no_rebuild flag to force build
- Reject invalid semver keys
- Order tags in AmbosoEnv maps as semver

### Changed

- do_build() returns early if target is found and no --force is provided
- do_build() calls make rebuild by default

## [0.1.5] - 2023-12-16

### Added

- Functionality for -s flag

- Lint only functionality for -x when -l is provided

### Fixed

- C header gen was missing some underscores

## [0.1.4] - 2023-12-15

### Added

- C header now includes:
  - head info for repo (somewhat compliant with original implementation)
  - commit message as a string (in extended header section)
- --no-color extension to turn off color output

- New -C flag to pass config file (contents used for config call during autotools prep)

### Fixed

- C header is no longer generating with "helapordo" in some places

### Changed

- Define EXPECTED_AMBOSO_API_LEVEL

- C header is no longer generated with flat 1.9.6

## [0.1.3] - 2023-12-14

### Changed

- Use semver cmp to determine latest tag for build subcommand

- Split main into modules

## [0.1.2] - 2023-12-13

### Added

- Functionality for subcommands
  - build to build latest tag for current mode
  - init to prep a new project

### Changed

- Mutability pass on env and args parameters, needed for build command last entry call

## [0.1.1] - 2023-12-10

### Added

- Functionality for -w flag
  - Report runtimes

### Changed

- Test record files now are expected with double extension (.k.std_)

## [0.1.0] - 2023-12-10

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
