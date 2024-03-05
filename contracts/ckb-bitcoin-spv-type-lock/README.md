# CKB Bitcoin SPV Type Lock

A type script for Bitcoin SPV clients which synchronize [Bitcoin] state into [CKB].

## Brief Introduction

A [Bitcoin] SPV in [CKB] contains a set of cells, this type script is used
to manage them.

Since this type script has a unique ID in its script [`args`], so the size of
the set of cells is immutable after they created.

### Cells

There are 2 kinds of cells in a Bitcoin SPV instance:

- Client Cell

  This cell is used to store the Bitcoin state.

  Each Bitcoin SPV instance should contain at least 1 client cell.

- Info Cell

  This cell is used to store the basic information of current Bitcoin SPV
  instance. Such as the ID of the tip client cell.

  Each Bitcoin SPV instance should contain only 1 info cell.

### Operations

There are 4 kinds of operations:

- Create

  Create all cells for a Bitcoin SPV instance in one transaction.

  The outputs of this transaction should contain 1 info cell and at least 1 client cell.

  In the follow part of this document, we denoted the number of client cells
  as `n`.

  In current implementation, it requires that cells must be continuous and
  in specified order:

  - The client info cell should be at the first.

  - Immediately followed by `n` client cells, and these cells should be
    ordered by their ID from smallest to largest.

  The structure of this kind of transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - ... ...
  Inputs:
  - Enough Capacity Cells
  Outputs:
  - SPV Info (last_client_id=0)
  - SPV Client (id=0)
  - SPV Client (id=1)
  - SPV Client (id=2)
  - ... ...
  - SPV Client (id=n-2)
  - SPV Client (id=n-1)
  - ... ...
  Witnesses:
  - SPV Bootstrap
  - ... ...
  ```

- Destroy

  All cells that use the same instance of this type lock should be destroyed
  together in one transaction.

  The structure of this kind of transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - ... ...
  Inputs:
  - SPV Info (last_client_id=0)
  - SPV Client (id=0)
  - SPV Client (id=1)
  - SPV Client (id=2)
  - ... ...
  - SPV Client (id=n-2)
  - SPV Client (id=n-1)
  - ... ...
  Outputs:
  - Unrelated Cell
  - ... ...
  Witnesses:
  - Unrelated Witness
  - ... ...
  ```

- Update

  After creation, the `n` client cells should have same data.

  The client cell who has the same ID as the `tip_client_id` in the info cell,
  we consider that it has the latest data.

  The client cell who has the next ID of the  `tip_client_id` in the info cell,
  we consider that it has the oldest data. The next ID of ID `n-1` is `0`.

  Once we update the Bitcoin SPV instance, we put the new data into the client
  cell which has the oldest data, and update the `tip_client_id` in the client
  info cell to its ID.

  Do the above step in repetition.

  The structure of this kind of transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - SPV Client (id=k)
  - ... ...
  Inputs:
  - SPV Client (id=k+1)
  - SPV Info (last_client_id=k)
  - ... ...
  Outputs:
  - SPV Client (id=k+1)
  - SPV Info (last_client_id=k+1)
  - ... ...
  Witnesses:
  - SPV Update
  - ... ...
  ```

- Reorg

  TODO

### Usages

When you want to verify a transaction with Bitcoin SPV Client cell:

- Choose any client cell which contains the block that transaction in.

- Create a transaction proof, with the following data:

  - The MMR proof of the header which contains this transaction.

  - The TxOut proof of that transaction.

  - The index of that transaction.

  - The height of that header.

- Use [the API `SpvClient::verify_transaction(..)`](https://github.com/yangby-cryptape/ckb-bitcoin-spv/blob/106e59ec53c2165b10c0e5a206dce7f2c0d1d2d6/verifier/src/types/extension/packed.rs#L255-L266) to verify the transaction.

  A simple example could be found in [this test](https://github.com/yangby-cryptape/ckb-bitcoin-spv/blob/106e59ec53c2165b10c0e5a206dce7f2c0d1d2d6/prover/src/tests/service.rs#L103-L119).

[Bitcoin]: https://bitcoin.org/
[CKB]: https://github.com/nervosnetwork/ckb
