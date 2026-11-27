# pcb-sch

`pcb-sch` provides the schematic and netlist data model used by the Zener
toolchain. It also reads and writes schematic positions stored in trailing
`pcb:sch` comments in `.zen` files.

## Schematic position comments

Each comment records the position and orientation of one schematic symbol:

```text
# pcb:sch <id> x=<f64> y=<f64> rot=<f64> [mirror=<x|y>]
```

`id` is a single-token symbol key. `x` and `y` are schematic coordinates, `rot`
is the rotation in degrees, and the optional `mirror` field specifies the `x` or
`y` reflection axis.

`x`, `y` and `rot` are plain decimals (`-12.5`); a line in any other form is
not a position.

The parser reads position comments from the whole file. A symbol written more
than once takes its position from the final contiguous comment block when it is
there, else from its last line; inside either, the last line wins. Blank lines
may occur inside the final block, and malformed comments are ignored. A save
gathers every position into the final block.

Netlist serialization writes positions to
`instances.<instance>.symbol_positions`. The `mirror` property is omitted when
the symbol is not mirrored.
