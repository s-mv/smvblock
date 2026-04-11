# smvblock

smvblock is a decentralized GPU compute network built around Rust nodes that
execute and propagate tensor computation tasks across a peer-to-peer mesh.

The system is designed as a minimal distributed execution layer where
computation is not centrally scheduled, but instead diffused across connected
nodes based on local capacity.

--------------------------------------------------------------------------------

## smvblock

smvblock is a distributed compute system where:

-   each node is a Rust process
-   each node can execute GPU compute locally
-   each node can forward tasks to peers
-   there is no central scheduler

The network behaves like a task diffusion system: nodes execute what they can
and propagate the rest.

--------------------------------------------------------------------------------

## core idea

A computation is represented as a task.

A task is: - sent to any node in the network - executed locally if capacity
allows - forwarded to peers if overloaded - terminated after a limited number of
hops (TTL)

This creates a decentralized execution model where computation naturally spreads
across available resources.

--------------------------------------------------------------------------------

## execution model

Each node maintains:

-   a list of peer nodes
-   a local execution capacity
-   a task processing loop

When a task is received:

1.  If the node has available capacity:
    -   execute the task locally using GPU compute (PyTorch backend via Rust
        bindings or modular runtime)
2.  Otherwise:
    -   forward the task to connected peers
3.  If no progress is possible:
    -   drop the task when TTL expires

--------------------------------------------------------------------------------

## build and run

### node (Rust)

Build the node:

    cargo build --release

Run a node:

    cargo run

To form a network, run multiple instances on different ports:

    cargo run -- --port 6767 
    cargo run -- --port 6768 
    cargo run -- --port 6769

Each node should be configured with a static peer list with `node/peers.json`
(for now).

Example peer list:

``` json
["192.168.29.42:6767", "192.168.29.42:6768", "127.168.29.42:6769"]
```

--------------------------------------------------------------------------------

## usage

The system is accessed through a Python client API.

### attach

Connect a client to an entry node in the network.

    client.attach("http://127.0.0.1:7000")

This node becomes the entry point for task submission.

--------------------------------------------------------------------------------

### deploy

Submit a computation task to the network.

    client.deploy({
        "op": "matmul",
        "a": [[1, 2], [3, 4]],
        "b": [[5, 6], [7, 8]],
        "ttl": 5
    })

The task will: - execute locally if possible - otherwise propagate through peers
until execution succeeds or TTL expires

--------------------------------------------------------------------------------

### detach

Disconnect the client from the entry node.

    client.detach()

This is optional and mainly used for cleanup or switching entry nodes.

--------------------------------------------------------------------------------

## task model

A task is a minimal computation unit.

    Task {
        id: string,
        op: string,
        payload: object,
        ttl: integer
    }

### current supported operations

-   matmul (initial MVP)

More operations will be added as the execution backend expands.

--------------------------------------------------------------------------------

## node behavior

Each node:

-   receives tasks over the network
-   evaluates local capacity
-   executes or forwards tasks
-   avoids revisiting exhausted paths using TTL only (no visited-set tracking)

Nodes are symmetric: there is no master node.

--------------------------------------------------------------------------------

## compute backend

Nodes execute GPU workloads locally using a modular backend.

Initial implementation: - PyTorch via Rust integration layer or subprocess
execution

This backend is intentionally abstracted to allow future replacement with: -
CUDA kernels - custom GPU runtimes - alternative tensor engines

--------------------------------------------------------------------------------

## limitations

This is an experimental system.

-   no strong consistency guarantees
-   no execution ordering guarantees across nodes
-   no fault recovery beyond TTL expiration
-   possible duplicate execution in edge cases
-   static peer configuration in MVP

--------------------------------------------------------------------------------

## goal

The goal of smvblock is to explore:

-   decentralized computation without central scheduling
-   task diffusion as a distributed execution strategy
-   minimal compute units over a peer network
-   GPU execution in a distributed, emergent system
