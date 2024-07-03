Benchmark done on [koliseo](https://github.com/jgabaut/koliseo) repo.

This is on default mode (git).

## NOTE:
 - Only one run was done, better averaging will be needed.
 - Comparing `invil 0.2.15` to `amboso 2.0.6`

To replicate this naive bench:
- Install amboso and invil as bin program
- cd `koliseo_git_repo`
- `time invil -i && time invil -p && time anvil -i && time anvil -p`

## Purge

| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.2.15| purge |real  0m0.519s|
| amboso 2.0.6| purge |real 0m55.506s|
| ------- | ------------------ | ------- |
| invil 0.2.3| purge |real  0m0.608s|
| amboso 2.0.0| purge |real 1m19.262s|

### Improvement

 - Runtime: `(-99.07%)`
 - Diff: `0m54.987s`


## Init


| Implementation | Operation          | Time |
| ------- | ------------------ | ------- |
| invil 0.2.15| init |real	4m0.708s|
| amboso 2.0.6| init |real	6m28.990s|
| ------- | ------------------ | ------- |
| invil 0.2.3| init |real	3m54.814s|
| amboso 2.0.0| init |real	6m56.630s|

### Improvement

 - Runtime: `(-38.12%)`
 - Diff: `2m28.282s`
