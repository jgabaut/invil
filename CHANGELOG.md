# Changelog

## [Unreleased]

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