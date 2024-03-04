# "Can Update Without Ownership" Lock

> [!WARNING]
> :warning: This contract is testing purpose only.

This contract is a mock contract, and it is used for testing.

The security of this contract is not guaranteed.

## Feature

This lock script is used to keep the total capacity of cells which use this
lock script could not be decreased, but any non-owner users could update
them.

## Brief Introduction

It will return success when any follow condition is satisfied:

- the witness for this lock script is reverse of the `args` in it.

- there is no witness for this lock script, but total capacity of cells
  which use this lock script are not greater than total capacity of cells
  which use this lock script after this transaction.
