# invil

## A Rust implementation of amboso, a simple build tool wrapping make.

## Table of Contents

+ [What is this thing?](#witt)
+ [Supported amboso features](#supported_amboso)
+ [Todo](#todo)

## What is this thing? <a name = "witt"></a>

  This is a Rust implementation of [amboso](https://github.com/jgabaut/amboso), a basic build tool wrapping make and upporting git tags.

  It's in a early stage, and there isn't any functionality yet.

## Supported amboso features <a name = "supported_amboso"></a>

  - Basic arguments parsing that complies with the bash implementation
    - Atm the `verbose` flag expects a `u8` argument, while bash parses multiple flag occurrences.
  - Same default for amboso directory (`./bin`).

## Todo <a name = "todo"></a>

  - Parse `stego.lock` with compatible logic to bash implementation
  - Check all runtime values are valid before op checks
  - Implement control flow for op checks
  - Implement quiet, verbose, silent functionality
  - Improve logging with a custom format
