Benchmark done on [koliseo](https://github.com/jgabaut/koliseo) repo.

This is on default mode (git).

## NOTE:
 - Only one run was done, better averaging will be needed.

To replicate this naive bench:
- Install amboso and invil as bin program
- cd `koliseo_git_repo`
- `time anvil -i && time anvil -p && time invil -i && time invil -p`

## Purge

| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.0.9| purge |real	0m0,617s|
| amboso 1.9.6| purge |real	0m32,945s|

### Improvement

 - Runtime: `(-98,13%)`
 - Diff: `32,328s`


## Init


| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.0.9| init |real	4m7,321s|
| amboso 1.9.6| init |real	6m35,594s|

### Improvement

 - Runtime: `(-37,48%)`
 - Diff: `2m28,273s`
