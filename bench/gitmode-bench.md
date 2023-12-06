Benchmark done on [koliseo](https://github.com/jgabaut/koliseo) repo.

This is on default mode (git).

## NOTE:
 - Only one run was done, better averaging will be needed.

To replicate this naive bench:
- Install amboso and invil as bin program
- cd `koliseo_git_repo`
- `time anvil -i`
- `time anvil -p`
- `time invil -i`
- `time invil -p`

## Purge

| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.0.8| purge |real	0m0,530s |
| amboso 1.9.6| purge |real	0m36,729s|

### Improvement

 - Runtime: `***(-98,55%)***`
 - Diff: `36,199s`


## Init


| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.0.8| init |real	4m9,765s|
| amboso 1.9.6| init |real	6m54,603s|

### Improvement

 - Runtime: `***(-39,75%)***`
 - Diff: `2m44,838s`
