import torch
import torch.fx as fx

OP_MAP_IDENTITY = {
    torch.matmul: "matmul",
    torch.add: "add",
    torch.sub: "sub",
}


def classify_node(node):
    target = node.target

    if target in OP_MAP_IDENTITY:
        return OP_MAP_IDENTITY[target]

    name = str(target)

    if "matmul" in name:
        return "matmul"

    if "add" in name:
        return "add"

    if "sub" in name:
        return "sub"

    if node.op == "call_method":
        if node.target == "__matmul__":
            return "matmul"
        if node.target == "__add__":
            return "add"
        if node.target == "__sub__":
            return "sub"

    return None


def lower_to_tasks(model, *example_inputs):
    gm = fx.symbolic_trace(model)

    tasks = []

    for node in gm.graph.nodes:

        if node.op not in ("call_function", "call_method"):
            continue

        op = classify_node(node)

        if op is None:
            continue

        inputs = []

        for arg in node.args:
            if isinstance(arg, fx.Node):
                inputs.append(arg.name)
            else:
                inputs.append(str(arg))

        tasks.append(
            {
                "op": op,
                "id": node.name,
                "inputs": inputs,
            }
        )

    return tasks
