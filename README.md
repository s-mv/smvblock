# smvblock

A lightweight, student-built Ethereum-lite blockchain implementation in Rust.

---

## Overview

**smvblock** is a minimalist Ethereum-lite blockchain built from the ground up in Rust — a hands-on project designed for deep understanding, not hype or fluff. It’s modular, fast, and clean: a no-nonsense playground to grasp the core mechanics of decentralized ledgers without drowning in complexity.

The codebase splits into three parts:

- **smv-core**: Core blockchain logic (blocks, transactions, cryptography).
- **node**: Full node implementation running the blockchain network, consensus,
  and storage.
- **client**: User-facing client to create transactions and interact with the
  network.

This is a zero-fee, zero-gas, educational prototype. Real crypto economics?
Not here. Just pure, unfiltered blockchain fundamentals.

---

## Features

- Modular Rust codebase with shared core library.
- Basic transaction and block structure.
- Simple consensus and networking (to be implemented).
- Block pruning to manage storage size.
- Web or CLI-based client interface.
- Designed for educational purposes: free and open, no gas fees or token
  economics.

---

## Getting Started

### Prerequisites

- Rust toolchain ([rustup](https://rustup.rs/))
- Cargo package manager (comes with Rust)
- ~~A terminal and a bit of patience.~~

### Build

```bash
# Clone the repository!
git clone https://github.com/s-mv/smvblock.git
cd smvblock

# Build all crates first.
cargo build
```

### Run Node

```bash
cargo run --bin node
```

### Run Client
```bash
cargo run --bin client
```

