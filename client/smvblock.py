import uuid
import torch

import grpc

import smvblock_pb2
import smvblock_pb2_grpc

from lower import lower_to_tasks

_channel = None
_stub = None
_entry_node = None


def attach(node_address: str):
    """
    Connect to a gRPC node.
    node_address: "localhost:7000"
    """

    global _channel, _stub, _entry_node

    _entry_node = node_address

    _channel = grpc.insecure_channel(node_address)
    _stub = smvblock_pb2_grpc.SmvNodeStub(_channel)

    print(f"[smvblock] attached to {node_address}")


def detach():
    global _channel, _stub, _entry_node

    if _channel:
        _channel.close()

    _channel = None
    _stub = None
    _entry_node = None

    print("[smvblock] detached")


def _encode_task(task: dict):
    return smvblock_pb2.Task(
        id=task["id"],
        op=task["op"],
        inputs=str(task["inputs"]),
        ttl=task["ttl"],
    )


def deploy(model, *example_inputs):
    """
    Lower model → tasks → send via gRPC
    """

    global _stub

    if _stub is None:
        raise RuntimeError("Client not attached")

    tasks = lower_to_tasks(model, *example_inputs)

    results = []

    for t in tasks:

        if "id" not in t:
            t["id"] = str(uuid.uuid4())

        if "ttl" not in t:
            t["ttl"] = 5

        req = _encode_task(t)

        # direct execution entry point
        res = _stub.Execute(req)

        results.append({"id": res.id, "status": res.status, "result": res.result})

    return results
