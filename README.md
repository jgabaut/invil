# invil
[![Latest version](https://img.shields.io/crates/v/invil.svg)](https://crates.io/crates/invil)

## A Rust implementation of amboso, a simple build tool wrapping make.

## Table of Contents

+ [What is this thing?](#witt)
+ [Supported amboso features](#supported_amboso)
+ [See how it behaves](#try_anvil)
+ [Todo](#todo)

## What is this thing? <a name = "witt"></a>

  This is a Rust implementation of [amboso](https://github.com/jgabaut/amboso), a basic build tool wrapping make and supporting git tags.

  It's in a early stage, and there is limited functionality.
  Check the next section for support info.

## Supported amboso features <a name = "supported_amboso"></a>

  - Basic arguments parsing that complies with the bash implementation
  - Same default for amboso directory (`./bin`).
  - Parse `stego.lock` with compatible logic to bash implementation
  - Base mode: init and build (only with basic command call)

  Flags support status:

  - [x] Basic env flags:  `-D`, `-K`, `-M`, `-S`, `-E`
  - [ ] Clock flag: `-C <startTime>`
  - [x] Linter mode: `-x`
    - [ ] Lint only: `-l`
    - [ ] Report lex: `-L`
  - [ ] C header gen mode: `-G`
  - [x] Verbose flag: `-V`
  - [ ] Test macro: `-t`
  - [ ] Test mode: `-T`
  - [ ] Git mode: `-g`
  - [x] Base mode: `-B` (only basic gcc call)
  - [x] Build: `-b` (only with -B)
  - [x] Run: `-r` (only with -B)
  - [x] Init: `-i` (only with -B)
  - [ ] Delete: `-d`
  - [ ] Purge: `-p`
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

## See how it behaves <a name = "try_anvil"></a>

To see how this marvelous work of art works, run:

```sh
  cd try-anvil
  ./try_anvil_auto
```
Refer to amboso info about this test script: [link](https://github.com/jgabaut/amboso#tryanvil)

Our version was slightly modified to actually make cargo build the release version of the binary we want to symlink to `anvil`.

## Todo <a name = "todo"></a>

  - Implement control flow for op checks
  - Implement silent functionality
  - Improve logging with a custom format
