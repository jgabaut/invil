# invil
[![Latest version](https://img.shields.io/crates/v/invil.svg)](https://crates.io/crates/invil)

## A Rust implementation of amboso, a simple build tool wrapping make.

## Table of Contents

+ [What is this thing?](#witt)
+ [Supported amboso features](#supported_amboso)
+ [Extended amboso features](#extended_amboso)
+ [See how it behaves](#try_anvil)
+ [Basic benchmark](#base_bench)
+ [Todo](#todo)

## What is this thing? <a name = "witt"></a>

  This is a Rust port of [amboso](https://github.com/jgabaut/amboso), a basic build tool wrapping make and supporting git tags.

  Invil can be used to:
  - Automate building a repo-curated list of git tagged versions (or also basic tagged versions with a full directory copy).
    - Ideally, the build command should be as short as `invil build`.
  - Run tests for a repo-curated directory with output comparison.
  - Generate new projects supporting the build tool using `invil init <DIR>`
  - Generate a basic header+impl containing project info, such as time of current commit

  It's (\*) on par with the original implementation, as of `amboso` `2.0.11`.
  Check the [next section](#supported_amboso) for more support info.
  Check [this section](#extended_amboso) for info about extensions to `amboso 2.0.4`.

  At the moment, only C projects are supported.
    - Check [this section](#extended_amboso) for info about the WIP `python` support.
    - The README still mostly refers only to the ambosoC kern usage.
  Different build modes are provided internally, depending on how full your autotool build support is:
  - Basic mode: a single `gcc` call. This may be expanded in a future version, to at least provide support for passing arguments to the compiler.
  - Make mode: for all tags higher than the version specified as providing make support, `invil` will expect a ready `Makefile` that correctly builds the target binary when `make` is called.
  - Automake mode: for all tags higher than the version specified as providing automake support, `invil` will expect a `Makefile.am` and a `configure.ac`, so that a `Makefile` with the same assumptions as Make mode can be generated.

  For more information on the `stego.lock` file, see the [amboso info](https://github.com/jgabaut/amboso#stego) about it.
  For more information on the `anvil` tool, see the [amboso wiki](https://github.com/jgabaut/amboso/wiki). Work in progress.

## Supported amboso features <a name = "supported_amboso"></a>

  - Basic arguments parsing that complies with the bash implementation
  - Same default for amboso directory (`./bin`).
  - Parse `stego.lock` with compatible logic to bash implementation
  - Base mode: full support
    - The original implementation itself does not expect autotools prep for base mode, but it can be done trivially.
  - Git mode: full support
    - The original implementation itself expects git mode tags to contain a `Makefile` in repo root.
  - C header gen: complete support (\*)
    - The original implementation print time as a pre-formatted string.
  - Test mode: complete support (\*)
    - Run executable found in test directories
    - Handle test macro flag to run on all valid queries
    - Record test output with `-b`
      - Not compliant with amboso <1.9.7 expectations: missing trailing `$`.
  - Passing configure arguments: complete support
    - Not compliant with amboso <1.9.9 expectations: -C flag was passing the arguments directly, not by reading a file.
  - Subcommands:
    - build    Quickly build latest version for current mode
    - init     Prepare new project with amboso
    - version  Print invil version

  - Note:
    - As of version `0.1.6`, by default `make rebuild` is called on build operation. This is the expected behaviour of `amboso` `2.x`. To revert to `1.x` original behaviour and call just `make`, run with `-R` or `--no-rebuild`.

  Flags support status:

  - [x] Basic env flags:  `-D`, `-K`, `-M`, `-S`, `-E`
  - [ ] Clock flag: `-Y <startTime>`
  - [x] Linter mode: `-x`
    - [x] Lint only: `-l`
    - [x] Report lex: `-L`
  - [x] C header gen mode: `-G` (detailed info is empty)
  - [x] Verbose flag: `-V`
  - [x] Test macro: `-t`
  - [x] Test mode: `-T`
  - [x] Git mode: `-g`
  - [x] Base mode: `-B`
  - [x] Build: `-b`
  - [x] Run: `-r`
  - [x] Init: `-i`
  - [x] Delete: `-d`
  - [x] Purge: `-p`
  - [x] Help: `-h`
  - [ ] Big Help: `-H`
  - [x] Version: `-v`
  - [x] List tags for current mode: `-l`
  - [x] List tags for git/base mode: `-L`
  - [x] Quiet flag: `-q`
  - [x] Watch flag: `-w`
  - [x] Warranty flag: `-W`
  - [x] Ignore gitcheck flag: `-X`
  - [x] Silent: `-s`
  - [x] Pass config argument: `-C`
  - [ ] Run make pack: `-z`
  - [x] No rebuild: `-R`
  - [x] Logged run: `-J`
    - Outputs to `./anvil.log`. Not backwards compatible with repos not ignoring the file explicitly.
  - [x] No color: `-P`
  - [x] Force build: `-F`
  - [x] Turn off extensions: `-e` (Only relative to 2.0.0)
  - [x] Pass CFLAGS to single file build mode: `-Z`
  - [x] Run make when no arguments are provided


## Extensions, relative to amboso 1.9.9

  - [x] When in `make` build mode, call `make rebuild` by default
    - [x] Add `--no-rebuild` to disable make rebuild and run just `make`
  - [x] Add `--logged` to output full log to file
    - Outputs to `./anvil.log`. Not backwards compatible with repos not ignoring the file explicitly.
  - [x] Add `-G` flag also includes:
    - a string for build OS (from `env::consts::OS`)
    - HEAD commit message
  - [x] Add `--no-color` to disable color output
  - [x] Add `--force` to overwrite ready targets

## Extensions to amboso 2.0

  - [x] Turn off extensions with `-e, --strict`
  - [x] Ignore missing repo in current work dir
  - [x] Add `-a` to set compatibility level
  - [x] Add `-k` to set project type
  - [x] Add `-O` to set stego.lock dir (defaults to working directory)
  - [x] Retrocompatible `stego.lock` parsing, up to `1.7.x`
  - [x] Init subcommand uses passed directory's basename for generated flags
  - [x] Read global config file from `$HOME/.anvil/anvil.toml`
  - [x] Add `-Z` to pass CFLAGS to single file build mode

## Extended amboso features <a name = "extended_amboso"></a>

## Experimental 2.1 version

  These features are experimental and subject to change.
  To enable them, add `--features="anvilPy"` to your build/install command.

  - [x] Use "anvilPy" kern to support python projects
    - Expects a suitable `pyproject.toml` is present alongside `stego.lock`
    - Experimental support for almost all flags
    - Only supported when provided from `stego.lock` itself
    - Example usage (make sure this is in your `stego.lock`):
      ```toml
      [ anvil ]
      version = "2.1.0"
      kern = "anvilPy"
      ```
  - [x] Refuse the experimental kern when running with `--strict`
    - The original implementation is not ready to support this extension.

## See how it behaves <a name = "try_anvil"></a>

To see how this marvelous work of art works, run:

```sh
  cd try-anvil
  ./try_anvil_auto
```
Refer to amboso info about this test script: [link](https://github.com/jgabaut/amboso#tryanvil)

Our version was slightly modified to actually make cargo build the release version of the binary we want to symlink to `anvil`.

## Basic benchmark <a name = "base_bench"></a>

Check out [this page](https://github.com/jgabaut/invil/blob/master/bench/gitmode-0.2.3-bench.md) for a very basic benchmark of runtime, relative to bash `amboso` implementation.

## Todo <a name = "todo"></a>

  - Extend original impl by handling autotools in base mode
  - Improve logging with a custom format
  - Improve horrendous git mode command chain
    - The current implementation is naive and just calls a bunch of `Command` to pipeline the git operations in a ugly iper-indented mess.
    - Resorting to shell commands is bad and defeats the purpose of this rewrite.
    - We have git2 crate to handle the git commands and should be able to reduce the amount of command wrapping.
