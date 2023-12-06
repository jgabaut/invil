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

  It's in a early stage, and there is limited functionality.
  Check the next section for support info.

## Supported amboso features <a name = "supported_amboso"></a>

  - Basic arguments parsing that complies with the bash implementation
  - Same default for amboso directory (`./bin`).
  - Parse `stego.lock` with compatible logic to bash implementation
  - Base mode: full support
    - The original implementation itself does not expect autotools prep for base mode, but it can be done trivially.
  - Git mode: full support
    - The original implementation itself expects git mode tags to contain a `Makefile` in repo root.
  - C header gen: basic support
    - The original implementation also prepares git commit info for the header.

  Flags support status:

  - [x] Basic env flags:  `-D`, `-K`, `-M`, `-S`, `-E`
  - [ ] Clock flag: `-C <startTime>`
  - [x] Linter mode: `-x`
    - [ ] Lint only: `-l`
    - [ ] Report lex: `-L`
  - [x] C header gen mode: `-G` (detailed info is empty)
  - [x] Verbose flag: `-V`
  - [ ] Test macro: `-t`
  - [ ] Test mode: `-T`
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
  - [ ] Watch flag: `-w`
  - [x] Warranty flag: `-W`
  - [x] Ignore gitcheck flag: `-X`


## Extensions

  - [x] `--logged` to output full log to file
    - Outputs to `./invil.log`. Not backwards compatible with repos not ignoring the file explicitly.
  - [x] `-G` flag also includes a string for build OS.
    - From `env::consts::OS`

## See how it behaves <a name = "try_anvil"></a>

To see how this marvelous work of art works, run:

```sh
  cd try-anvil
  ./try_anvil_auto
```
Refer to amboso info about this test script: [link](https://github.com/jgabaut/amboso#tryanvil)

Our version was slightly modified to actually make cargo build the release version of the binary we want to symlink to `anvil`.

## Basic benchmark <a name = "base_bench"></a>

Check out [this page](https://github.com/jgabaut/invil/bench/gitmode-bench.md) for a very basic benchmark of runtime, relative to bash `amboso` implementation.

## Todo <a name = "todo"></a>

  - Implement test mode
  - Implement silent functionality
  - Extend original impl by handling autotools in base mode
  - Improve logging with a custom format
  - Improve horrendous git mode command chain
    - The current implementation is naive and just calls a bunch of `Command` to pipeline the git operations in a ugly iper-indented mess.
    - Resorting to shell commands is bad and defeats the purpose of this rewrite.
    - We have git2 crate to handle the git commands and should be able to reduce the amount of command wrapping.
  - Add detailed git info for C header generation
