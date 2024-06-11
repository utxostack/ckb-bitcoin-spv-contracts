
# CKB Bitcoin SPV Type Lock

A type script designed for Bitcoin SPV clients ensures the valid synchronization of the [Bitcoin] blockchain state into the Nervos [CKB] network.

## Brief Introduction

A Bitcoin  SPV on CKB consists of cells that are managed by the CKB Bitcoin SPV Type Lock and identified by the script args. The number of live cells with the script args remains fixed once created, and these cells will be destroyed collectively as a group.

### Cells

A Bitcoin SPV instance contains two types of cells: **SPV info cell** and **SPV client cell**.

- **SPV Client Cell**

  A cell is identified as an SPV client cell if its type script matches the SPV type script.

  SPV client cells store the Bitcoin state. Each Bitcoin SPV instance includes a minimum of three SPV client cells.

  ```yaml
  Client Cell:  
    Type Script:
      code hash: "..."
      hash type: "type"
      args: "typeid + clients count + flags"
    Data:
      - id
      - btc tip block hash
      - btc headers mmr root
      - target adjust info
  ```

- **SPV Info Cell**

  The SPV info cell stores the basic information of the current Bitcoin SPV instance, such as `tip_ client_cell_id`. Each Bitcoin SPV
  instance contains only one SPV info cell.

  ```yaml
  Info Cell:
    Type Script:
      code hash: "..."
      hash type: "type"
      args: "typeid + clients count + flags"
    Data: 
      - tip client cell id
  ```

### Operations

There are 4 kinds of operations in the Bitcoin SPV type script:

- **Create**

  This operation initiates all necessary cells for a Bitcoin SPV instance in a single transaction.

  The `outputs` include one SPV info cell and at least one SPV client cell. Cells should be consecutive, with the SPV info cell first,
  followed by N SPV client cells ordered by their ID from smallest to largest. 

  Let's denote the number of SPV client cells as `n`. The structure of this transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - ... ...
  Inputs:
  - Enough Capacity Cells
  Outputs:
  - SPV Info (tip_client_id=0)
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

- **Destroy**

  Cells within a single Bitcoin SPV instance should be destroyed in one transaction..

  The structure of this kind of transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - ... ...
  Inputs:
  - SPV Info (tip_client_id=0)
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

- **Update**

  The SPV client cell which ID matches the `tip_client_id` of the SPV info cell contains the most recent data, the SPV client cell next
  in the sequence after the `tip_client_id` of the info cell holds the oldest data. This sequence arrangement of cells forms a ring
  where after the last cell (`ID = n-1`), it wraps around back to the first cell (`ID = 0`).

  When the Bitcoin SPV instance is updated, the new data will be put into the client cell that currently has the oldest data. Also,
  the `tip_client_id` in the SPV info cell will be replaced by the `ID` of the SPV client cell that just received the new data. This SPV
  info cell now becomes the new "latest data" holder.

  The structure of this kind of transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - SPV Client (id=k)
  - ... ...
  Inputs:
  - SPV Info (tip_client_id=k)
  - SPV Client (id=k+1)
  - ... ...
  Outputs:
  - SPV Info (tip_client_id=k+1)
  - SPV Client (id=k+1)
  - ... ...
  Witnesses:
  - SPV Update
  - ... ...
  ```

- **Reorg**

  When receiving blocks from a new, longer chain, if the last common ancestor of both the old and new chains is identified by the [tip]
  of client cell, a reorg is triggered. This reorg starts from this last common ancestor and rearranges the client cells accordingly.

  **If no common ancestor block is identified, the Bitcoin SPV instance will fail and require re-deployment.**

  Let's denote the client ID of the best common ancestor as `t`. The structure of this transaction is as follows:

  ```yaml
  Cell Deps:
  - Type Lock
  - SPV Client (id=t)
  - ... ...
  Inputs:
  - SPV Info (tip_client_id=k)
  - SPV Client (id=t+1)
  - SPV Client (id=t+2)
  - SPV Client (id=...)
  - SPV Client (id=k)
  - ... ...
  Outputs:
  - SPV Info (tip_client_id=t+1)
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

To verify a transaction using the Bitcoin SPV Client cell, follow these steps:

- Select an SPV client cell that contains the block where the transaction is;

- Create a transaction proof, with the following data:

  - The MMR proof of the block header which contains this transaction;

  - The TxOut proof of the transaction;

  - The index of the transaction;

  - The height of the block header.

- Use the SpvClient::verify_transaction(..) for the verification. For detailed guidance, please refer to the [API example].

### Limits

- The minimum count of SPV client cells is 3;
  
- While there is no fixed maximum count of SPV client cells; it is advisable not to exceed `250` given the **`u8`** data type.

### Known Issues and Solutions

- **Issue #2**: `VM Internal Error: MemWriteOnExecutablePage`

  **Solution**: Don't set hash type[^1] to be `Data`.

  `Data1` is introduced in [CKB RFC 0032], and `Data2` is introduced in [CKB RFC 0051].

- **Issue #2**: Failed to reorg when there is only 1 stale SPV client.

  **Solution**: When only one SPV client cell is stale, a typical reorg transaction has the same structure as an update transaction,
  consisting of one SPV client cell in the inputs and one SPV client cell in the outputs. However, this similarity can lead to ambiguity.

  To address this issue, the following rules have been set: 

    - In cases where only one SPV client has failed, the reorg transaction must involve the reconstruction of one additional SPV client;
    Specifically, the reorg transaction for one stale SPV client should include two SPV client cell in the `inputs` and two SPV client
    cell in the `outputs` ;
    - Considering that reorgs are a rare occurrence on the Bitcoin mainnet, the cost incurred by this approach is considered manageable.

- **Issue #3**: Throw **"Arithmetic Operation Overflow"** when updating a Bitcoin SPV instance for a Bitcoin dev chain.
    
    **Solution**: As the Bitcoin dev chain does not adhere to Bitcoin difficulty adjustment, calculations for the next target and the
    partial chain work could result in an arithmetic overflow.
    
    
[^1]: [Section "Code Locating"] in "CKB RFC 0022: CKB Transaction Structure".

[Bitcoin]: https://bitcoin.org/
[CKB]: https://github.com/nervosnetwork/ckb

[`args`]: https://github.com/nervosnetwork/rfcs/blob/v2020.01.15/rfcs/0019-data-structures/0019-data-structures.md#description-1
[the field `output_type` of `WitnessArgs`]: https://github.com/nervosnetwork/ckb/blob/v0.114.0/util/gen-types/schemas/blockchain.mol#L117

[API example]: https://github.com/ckb-cell/ckb-bitcoin-spv/blob/2464c8f/prover/src/tests/service.rs#L132-L181


[CKB RFC 0022]:https://github.com/nervosnetwork/rfcs/blob/v2020.01.15/rfcs/0022-transaction-structure/0022-transaction-structure.md#code-locating
[Section "Code Locating"]: https://github.com/nervosnetwork/rfcs/blob/v2020.01.15/rfcs/0022-transaction-structure/0022-transaction-structure.md#code-locating
[CKB RFC 0032]: https://github.com/nervosnetwork/rfcs/blob/dff5235616e5c7aec706326494dce1c54163c4be/rfcs/0032-ckb-vm-version-selection/0032-ckb-vm-version-selection.md#specification
[CKB RFC 0051]: https://github.com/nervosnetwork/rfcs/blob/dff5235616e5c7aec706326494dce1c54163c4be/rfcs/0051-ckb2023/0051-ckb2023.md#ckb-vm-v2
