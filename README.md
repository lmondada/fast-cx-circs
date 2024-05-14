### Usage

Make sure to run in release mode:
```
cargo run --release -- -t TARGET_CIRC -m ALLOWED_CX
```

- `TARGET_CIRC` is the file with the target circuit to resynthesise.
- `ALLOWED_CX` is the file with the allowed CX interactions.

Both `TARGET_CIRC` and `ALLOWED_CX` assume that the file is composed of lines with two integers on each:
the control and target qubit of each CX gate.
See `data/target_circuit_23a` and `data/layout_4_all_to_all` for example files.

### Stabiliser support

Using `-a astar-stabiliser` you can also synthesise a new circuit that
maps a source stabiliser state to a target stabiliser state.
In this case both `--source` and `--target` file names are required. The
files should be lines of pauli strings in the X basis, e.g. `IXIIIX`.

### Help
```
Find optimal CX circuits, fast.

Usage: fast-cx-circs [OPTIONS]

Options:
  -t, --target <TARGET>  Name of target circuit or state [default: in]
  -s, --source <SOURCE>  Name of source circuit or state. For circuits, defaults to identity
  -m, --moves <MOVES>    Name of moves file [default: all_to_all]
  -o, --output <OUTPUT>  Name of output file [default: out]
  -d, --depth <DEPTH>    Maximum depth of BFS. The maximum gate count will be 3*depth. Warning: I do not recommend setting this value higher than 5, memory consumption goes through the roof [default: 5]
  -a, --algo <ALGO>      [default: astar] [possible values: mitm, astar, astar-stabiliser]
  -h, --help             Print help (see more with '--help')
  -V, --version          Print version
```