# smv-core — The Core Blockchain Engine

This crate is the foundation of the smvblock. It implements all required
components of a blockchain, focusing on security and simplicity.

It's a minimal but fully functional blockchain core that anyone with basic
Rust knowledge and an interest in blockchain can understand and extend.

---

## Core Concepts, Modules

### Blockchain

- Manages the chain of blocks, ensuring blocks are linked correctly by hashes.
- Holds the current pool of pending transactions.
- Validates transactions before adding them to the pool.
- Mines new blocks by finding a valid nonce (proof-of-work).
- Validates the entire chain’s consistency and integrity.

### Block

- Contains a vector of transactions.
- References the previous block via a cryptographic hash.
- Stores a nonce for proof-of-work.
- Verifies its own transactions and integrity when created or received.

### Transaction

- Represents a token transfer from one account to another.
- Uses cryptographic signatures to prove authenticity.
- Contains a nonce to prevent replay attacks and ensure order.
- Amount and fee values are carefully checked against the sender’s balance.

### State

- Holds all account balances and nonces.
- Enforces that no transaction overspends.
- Updates balances and nonces atomically when transactions are applied.
- This is the ledger’s source of truth.

### Crypto

- Generates public/private keypairs.
- Signs and verifies transactions.
- Hashes blocks and transactions.
- Derives addresses from public keys.

---

## How It Works — The Flow

1. **Accounts** (e.g., Pikachu and Geodude) start with zero balance by default.

2. **Balances and nonces** live in the `State` struct. No money exists until
   the genesis block creates some.

3. Transactions are created by signing a message with the sender’s private key,
   including:
   - Recipient address
   - Amount to transfer
   - Sender’s current nonce (must be exactly one more than previous)

4. `Blockchain::add_transaction` validates the transaction against the `State`:
   - Checks sender’s balance covers amount + fee
   - Checks nonce matches
   - Rejects if validation fails (e.g., `InsufficientBalance`)

5. Valid transactions go into a **pending pool** waiting to be mined.

6. When mining a block:
   - The blockchain collects pending transactions.
   - Applies proof-of-work by finding a nonce satisfying the difficulty.
   - Updates the `State` with all transactions in the new block.
   - Adds the new block to the chain.

7. Every new block links to the previous one via hash, forming an immutable
   ledger.

---

## Some Important Aspects

- **Genesis block:** The only place where initial balances are minted. This is
  manually coded to give Pikachu some starting SMVs so the chain can bootstrap.

- **Balances and nonces are private** inside `State` to prevent accidental
  external mutation. All interaction happens through controlled APIs ensuring
  correctness.

- **Transactions require strict nonce order** in order to avoid replay or
  double-spending.

- The **proof-of-work** mining ensures adding blocks is computationally
  expensive, securing the chain against tampering.

Example:

- Pikachu sends 100 SMV tokens to Geodude.
- The transaction is signed with Pikachu’s private key.
- If Pikachu has enough balance and the nonce is correct, the transaction is
  accepted.
- After mining a block including this transaction, Geodude’s balance
  increases by 100 SMV.

---

## Why smv-core?

This crate is designed to be:

- **Simple** — no unnecessary dependencies or abstractions.
- **Clear** — you can trace all state changes explicitly.
- **Secure** — strict checks prevent invalid states or transactions.
- **Educational** — perfect for developers who want to *build* and *understand*
  blockchain from scratch.
