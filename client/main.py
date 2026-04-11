import torch
import torch.nn as nn
import smvblock


class Model(nn.Module):
    def forward(self, a, b):
        c = a @ b
        d = c + 1
        return d


model = Model()


smvblock.attach("127.0.0.1:7000")


result = smvblock.deploy(
    model,
    torch.randn(2, 2),
    torch.randn(2, 2),
)


print("\n=== LOWERED OUTPUT ===\n")

for t in result["tasks"]:
    print(t)


smvblock.detach()
