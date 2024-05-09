### Usage

Make sure to run in release mode:
```
cargo run --release -- -i INPUT_CIRC -m ALLOWED_CX
```

- `INPUT_CIRC` is the input circuit file.
- `ALLOWED_CX` is the CX interactions allowed in the output circuit.

Both `INPUT_CIRC` and `ALLOWED_CX` assume that the file is composed of lines with two integers on each:
the control and target qubit of each CX gate. See "in" and "moves" for example files.

###

Help:
```
Find optimal CX circuits, fast.

Usage: fast-cx-circs [OPTIONS]

Options:
  -i, --input <INPUT>    Name of input circuit file [default: in]
  -m, --moves <MOVES>    Name of moves file [default: all_to_all]
  -o, --output <OUTPUT>  Name of output file [default: out]
  -d, --depth <DEPTH>    Maximum depth of BFS [default: 5]
  -h, --help             Print help
  -V, --version          Print version
```