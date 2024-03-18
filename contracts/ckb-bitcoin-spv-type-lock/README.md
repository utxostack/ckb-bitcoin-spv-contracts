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
  - SPV Info (last_client_id=k)
  - SPV Client (id=k+1)
  - ... ...
  Outputs:
  - SPV Info (last_client_id=k+1)
  - SPV Client (id=k+1)
  - ... ...
  Witnesses:
  - SPV Update
  - ... ...
  ```

- Reorg

  When receives blocks from a new longer chain, and there has at least one
  client cell whose tip block is the common ancestor block of both the old
  chain and the new chain, then a chain reorganization will be required.

  **If no common ancestor block was found, then the Bitcoin SPV instance
  will be broken, and it requires re-deployment.**

  let's denote the client ID of the best common ancestor to be `t`.

  The structure of this kind of transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - SPV Client (id=t)
  - ... ...
  Inputs:
  - SPV Info (last_client_id=k)
  - SPV Client (id=t+1)
  - SPV Client (id=t+2)
  - SPV Client (id=...)
  - SPV Client (id=k)
  - ... ...
  Outputs:
  - SPV Info (last_client_id=t+1)
  - SPV Client (id=t+1)
  - SPV Client (id=t+2)
  - SPV Client (id=...)
  - SPV Client (id=k)
  - ... ...
  Witnesses:
  - SPV Update
  - ... ...
  ```

For all operations, the witness for Bitcoin SPV should be set at the same
index of the output SPV info cell, and the proof should be set in
[the field `output_type` of `WitnessArgs`].

### Usages

When you want to verify a transaction with Bitcoin SPV Client cell:

- Choose any client cell which contains the block that transaction in.

- Create a transaction proof, with the following data:

  - The MMR proof of the header which contains this transaction.

  - The TxOut proof of that transaction.

  - The index of that transaction.

  - The height of that header.

- Use [the API `SpvClient::verify_transaction(..)`](https://github.com/ckb-cell/ckb-bitcoin-spv/blob/2464c8f/verifier/src/types/extension/packed.rs#L275-L292) to verify the transaction.

  A simple example could be found in [this test](https://github.com/ckb-cell/ckb-bitcoin-spv/blob/2464c8f/prover/src/tests/service.rs#L132-L181).

### Known Issues

- `VM Internal Error: MemWriteOnExecutablePage`

  Don't set hash type[^1] to be `Data`.

  `Data1` is introduced from [CKB RFC 0032], and `Data2` is introduced from [CKB RFC 0051].

[^1]: [Section "Code Locating"] in "CKB RFC 0022: CKB Transaction Structure".

[Bitcoin]: https://bitcoin.org/
[CKB]: https://github.com/nervosnetwork/ckb

[`args`]: https://github.com/nervosnetwork/rfcs/blob/v2020.01.15/rfcs/0019-data-structures/0019-data-structures.md#description-1
[the field `output_type` of `WitnessArgs`]: https://github.com/nervosnetwork/ckb/blob/v0.114.0/util/gen-types/schemas/blockchain.mol#L117

[Section "Code Locating"]: https://github.com/nervosnetwork/rfcs/blob/v2020.01.15/rfcs/0022-transaction-structure/0022-transaction-structure.md#code-locating
[CKB RFC 0032]: https://github.com/nervosnetwork/rfcs/blob/dff5235616e5c7aec706326494dce1c54163c4be/rfcs/0032-ckb-vm-version-selection/0032-ckb-vm-version-selection.md#specification
[CKB RFC 0051]: https://github.com/nervosnetwork/rfcs/blob/dff5235616e5c7aec706326494dce1c54163c4be/rfcs/0051-ckb2023/0051-ckb2023.md#ckb-vm-v2
