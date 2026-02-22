# ResNet-tiny MNIST — SPIR-V compute kernels (CLFC)

This directory contains **6 general-purpose Vulkan compute shaders** for the Coelanox ISA, packed into a single **CLF Compute** file (`.clfc`) for use by the Coelanox packager when building the container code section for ResNet-tiny MNIST.

## Op_ids and kernels

| Op             | op_id | Shader        | Description |
|----------------|-------|---------------|-------------|
| Add            | 1     | add.comp      | Element-wise add: `output[i] = input_a[i] + input_b[i]` |
| Relu           | 10    | relu.comp     | ReLU: `output[i] = max(0, input[i])` |
| Convolution    | 30    | conv.comp     | 2D NCHW convolution (configurable kernel, stride, padding) |
| GlobalAvgPool  | 34    | globalavgpool.comp | Global average pool: one value per (N,C) over H,W |
| BatchNorm      | 35    | batchnorm.comp| Batch norm: `(input - mean) * inv_std * gamma + beta` |
| MatMul         | 50    | matmul.comp   | Matrix multiply (+ optional bias): `output = input @ weights + bias` |

All kernels are **parameterized** via push constants and descriptor bindings; no hardcoded sizes. The runtime binds buffers and push constants from the container/IR at dispatch time.

## Build

**Requirements**

- **glslc** (Vulkan SDK) or **glslangValidator** (glslang) to compile GLSL → SPIR-V
- **Rust** and **cargo** (CLF repo) for the packer

**Steps**

```bash
# From this directory (CLF/mnist)
./build.sh
```

This will:

1. Compile each `shaders/*.comp` to `spv/*.spv`
2. Run the CLF packer to produce `resnet_tiny_mnist.spv.clfc`

Output: **`resnet_tiny_mnist.spv.clfc`** in this directory.

## Packer invocation (reference)

The build script runs:

```bash
cargo run --bin coelanox-packer -- \
  --vendor "CLF-MNIST-SPIRV" \
  --target "SPIR-V" \
  --kind compute \
  --output resnet_tiny_mnist.spv.clfc \
  --align 16 \
  1:spv/add.spv \
  10:spv/relu.spv \
  30:spv/conv.spv \
  34:spv/globalavgpool.spv \
  35:spv/batchnorm.spv \
  50:spv/matmul.spv
```

Using **`.clfc`** ensures the file is treated as a **Compute** CLF (see SPEC.md §3.1.1).

## Discovery (Coelanox backend)

Point the Coelanox backend manager at this directory (or the path to `resnet_tiny_mnist.spv.clfc`). The packager will use this CLF when building a ResNet-tiny container for the **SPIR-V** target. Each node in execution order is mapped to its op_id; the packager looks up the blob in the CLF manifest and appends it to the code section.

## Kernel contract (summary)

- **Buffers:** Descriptor set 0, bindings as per each shader (input(s), weights, output; optional bias).
- **Sizes/config:** Push constants (element count, N/C/H/W, M/K/N, kernel size, stride, padding, etc.).
- **Precision:** FP32. Flat float arrays; layout matches Coelanox scalar (NCHW where applicable) for correctness.
- **Execution:** One dispatch per node in node order; runtime binds the correct buffers and push constants for that node.

## Scalar reference

Math and layout match the Coelanox scalar implementations in `coelanox-core/src/scalar/` (arithmetic, activations, matrix_ops, normalization, pooling) so that CLF and scalar give the same results for the same container.
