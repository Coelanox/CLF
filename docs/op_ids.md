# CLF Op ID Registry (canonical)

Coelanox defines a **stable op_id** set so that the packager and any CLF producer agree on meaning. All multi-byte values are little-endian in the file format; this table is the semantic registry.

| op_id | Name / Coelanox OpType | Category    |
|-------|------------------------|-------------|
| 0     | Reserved / unknown / custom | —           |
| 1     | Add                    | Arithmetic  |
| 2     | Subtract               | Arithmetic  |
| 3     | Multiply               | Arithmetic  |
| 4     | Divide                 | Arithmetic  |
| 5–9   | (reserved arithmetic)  | Arithmetic  |
| 10    | Relu                   | Activations |
| 11    | Sigmoid                | Activations |
| 12    | Tanh                   | Activations |
| 13    | Softmax                | Activations |
| 14    | LogSoftmax             | Activations |
| 15    | Gelu                   | Activations |
| 16    | Swish                  | Activations |
| 17–19 | (reserved activations) | Activations |
| 20    | Sqrt                   | Math        |
| 21    | Pow                    | Math        |
| 22    | Cos                    | Math        |
| 23    | Sin                    | Math        |
| 24    | Exp                    | Math        |
| 25    | Log                    | Math        |
| 26–29 | (reserved math)        | Math        |
| 30    | Convolution            | Conv/pool/norm |
| 31    | MaxPool                | Conv/pool/norm |
| 32    | AvgPool                | Conv/pool/norm |
| 33    | GlobalMaxPool          | Conv/pool/norm |
| 34    | GlobalAvgPool          | Conv/pool/norm |
| 35    | BatchNorm              | Conv/pool/norm |
| 36    | LayerNorm              | Conv/pool/norm |
| 37    | Dropout                | Conv/pool/norm |
| 38–39 | (reserved)             | Conv/pool/norm |
| 40    | Reshape                | Tensor manip |
| 41    | Transpose              | Tensor manip |
| 42    | Permute                | Tensor manip |
| 43    | Concatenate            | Tensor manip |
| 44    | Split                  | Tensor manip |
| 45    | Slice                  | Tensor manip |
| 46    | Gather                 | Tensor manip |
| 47    | Scatter                | Tensor manip |
| 48–49 | (reserved)             | Tensor manip |
| 50    | MatMul                 | Linear      |
| 51    | Gemm                   | Linear      |
| 52–59 | (reserved)             | Linear      |
| 60    | ReduceSum              | Reductions  |
| 61    | ReduceMean             | Reductions  |
| 62    | ReduceMax              | Reductions  |
| 63    | ReduceMin              | Reductions  |
| 64    | ReduceProd             | Reductions  |
| 65–69 | (reserved)             | Reductions  |
| 70–79 | Broadcast / expand     | Broadcast   |
| 80    | Equal                  | Comparisons |
| 81    | NotEqual               | Comparisons |
| 82    | Greater                | Comparisons |
| 83    | GreaterEqual           | Comparisons |
| 84    | Less                   | Comparisons |
| 85    | LessEqual              | Comparisons |
| 86–89 | (reserved)             | Comparisons |
| 90    | And                    | Logical     |
| 91    | Or                     | Logical     |
| 92    | Not                    | Logical     |
| 93–99 | (reserved)             | Logical     |
| 100+  | Reserved or custom     | —           |

**Usage:**

- **Packager (consumer):** Map each IR node’s `OpType` to `op_id` via `op_type_to_clf_id(OpType)`, then look up the blob in the CLF manifest.
- **Producer:** Use this table to assign each compiled kernel to the correct op_id when building the `.clf` (e.g. in the packer input manifest).
