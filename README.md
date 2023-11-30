# invil

## A Rust implementation of amboso, a simple build tool wrapping make.

## Table of Contents

+ [What is this thing?](#witt)
+ [Supported amboso features](#supported_amboso)
+ [See how it behaves](#try_anvil)
+ [Todo](#todo)

## What is this thing? <a name = "witt"></a>

  This is a Rust implementation of [amboso](https://github.com/jgabaut/amboso), a basic build tool wrapping make and supporting git tags.

  It's in a early stage, and there isn't any functionality yet.

## Supported amboso features <a name = "supported_amboso"></a>

  - Basic arguments parsing that complies with the bash implementation
  - Same default for amboso directory (`./bin`).
  - Parse `stego.lock` with compatible logic to bash implementation

## See how it behaves <a name = "try_anvil"></a>

To see how this marvelous work of art works, run:

```sh
  cd try-anvil
  ./try_anvil_auto
```
Refer to amboso info about this test script: [link](https://github.com/jgabaut/amboso#tryanvil)

Our version was slightly modified to actually make cargo build the release version of the binary we want to symlink to `anvil`.

## Todo <a name = "todo"></a>

  - Check all runtime values are valid before op checks
  - Implement control flow for op checks
  - Implement quiet, verbose, silent functionality
  - Improve logging with a custom format
