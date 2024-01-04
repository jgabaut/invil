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
| invil 0.2.3| purge |real  0m0.608s|
| amboso 2.0.0| purge |real 1m19.262s|

### Improvement

 - Runtime: `(-98.76%)`
 - Diff: `1m18.654s`


## Init


| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.2.3| init |real	3m54.814s|
| amboso 2.0.0| init |real	6m56.630s|

### Improvement

 - Runtime: `(-43.69%)`
 - Diff: `3m1.816s`
