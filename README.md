# invil
[![Latest version](https://img.shields.io/crates/v/invil.svg)](https://crates.io/crates/invil)

## A Rust implementation of amboso, a simple build tool wrapping make.

## Table of Contents

+ [What is this thing?](#witt)
+ [Supported amboso features](#supported_amboso)
+ [See how it behaves](#try_anvil)
+ [Basic benchmark](#base_bench)
+ [Todo](#todo)

## What is this thing? <a name = "witt"></a>

  This is a Rust implementation of [amboso](https://github.com/jgabaut/amboso), a basic build tool wrapping make and supporting git tags.

  It's almost on par with the original implementation, as of amboso 1.9.7.
  Check the next section for support info.

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
  - Passing configure arguments: (\*)
    - Amboso 1.9.8 expects -C flag to be passing the arguments directly, not by reading a file.
  - Subcommands:
    - build    Quickly build latest version for current mode
    - init     Prepare new project with amboso
    - version  Print invil version

  Flags support status:

  - [x] Basic env flags:  `-D`, `-K`, `-M`, `-S`, `-E`
  - [ ] Clock flag: `-Y <startTime>`
  - [x] Linter mode: `-x`
    - [ ] Lint only: `-l`
    - [ ] Report lex: `-L`
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
  - [ ] CFG flag: `-c`
  - [x] Watch flag: `-w`
  - [x] Warranty flag: `-W`
  - [x] Ignore gitcheck flag: `-X`
  - [ ] Silent: `-s`
  - [ ] Pass config argument: `-C`


## Extensions

  - [x] `--logged` to output full log to file
    - Outputs to `./invil.log`. Not backwards compatible with repos not ignoring the file explicitly.
  - [x] `-G` flag also includes:
    - a string for build OS (from `env::consts::OS`)
    - HEAD commit message
  - [x] `--no-color` to disable color output

## See how it behaves <a name = "try_anvil"></a>

To see how this marvelous work of art works, run:

```sh
  cd try-anvil
  ./try_anvil_auto
```
Refer to amboso info about this test script: [link](https://github.com/jgabaut/amboso#tryanvil)

Our version was slightly modified to actually make cargo build the release version of the binary we want to symlink to `anvil`.

## Basic benchmark <a name = "base_bench"></a>

Check out [this page](https://github.com/jgabaut/invil/blob/master/bench/gitmode-0.0.9-bench.md) for a very basic benchmark of runtime, relative to bash `amboso` implementation.

## Todo <a name = "todo"></a>

  - Implement silent functionality
  - Extend original impl by handling autotools in base mode
  - Improve logging with a custom format
  - Improve horrendous git mode command chain
    - The current implementation is naive and just calls a bunch of `Command` to pipeline the git operations in a ugly iper-indented mess.
    - Resorting to shell commands is bad and defeats the purpose of this rewrite.
    - We have git2 crate to handle the git commands and should be able to reduce the amount of command wrapping.
  - Add detailed git info for C header generation
