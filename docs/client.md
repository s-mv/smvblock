# Client Commands

The client provides a set of commands to interact with the blockchain network. Below are the commands related to peers:

---

## Commands

### Add Peer

Add a peer to the node's P2P network.

```bash
cargo run --bin client -- add-peer --node <NODE_URL> --peer <PEER_ADDRESS>
```

- `NODE_URL`: The URL of the node to connect to.
- `PEER_ADDRESS`: The address of the peer to add.

---

### List Peers

List all peers connected to the node.

```bash
cargo run --bin client -- list-peers --node <NODE_URL>
```

- `NODE_URL`: The URL of the node to query.

---

### Get Status

Retrieve the status of a node.

```bash
cargo run --bin client -- status --node <NODE_URL>
```

- `NODE_URL`: The URL of the node to query.

---

### Add Peer (Node)

Add a peer to the node's P2P network directly via the `node` binary.

```bash
cargo run --bin node --mode <MODE> --add-peer <PEER_ADDRESS>
```

- `MODE`: The mode of the node (`seed`, `normal`, or `shallow`).
- `PEER_ADDRESS`: The address of the peer to add.

---

### List Peers (Node)

List all peers connected to the node directly via the `node` binary.

```bash
cargo run --bin node --mode <MODE> --list-peers
```

- `MODE`: The mode of the node (`seed`, `normal`, or `shallow`).

---
