### Usage

```
cargo run -- -i INPUT_CIRC -m ALLOWED_CX
```

- `INPUT_CIRC` is the input circuit file.
- `ALLOWED_CX` is the CX interactions allowed in the output circuit.

Both `INPUT_CIRC` and `ALLOWED_CX` assume that the file is composed of lines with two integers on each:
the control and target qubit of each CX gate. See "in" and "moves" for example files.

