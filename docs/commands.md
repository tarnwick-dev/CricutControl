# Command layer: design to cut

Turns an SVG design into the command stream a machine cuts. The geometry is
solid; the exact machine wire bytes of the draw commands are a documented
hypothesis until confirmed against a decrypted full cut.

## Pipeline

```
SVG path "d"  --cricut-design-->  polylines (inches)
              --cricut-bridge::cut-->  draw records (MoveTo/LineTo/EndPath)
              --cricut-proto::wire-->  command frames (opcode 0x42 + records)
              --cricut-bridge::session-->  encrypted + sent over RFCOMM
```

Try it without a machine:

```
cricut plan "M0,0 L72,0 L72,72 L0,72 Z"   # a 1-inch square (points, 72/inch)
```

## What is solid

- `cricut-design`: SVG path parsing and curve flattening to polylines in inches.
  Standard geometry, no reverse engineering. Set the source units (Design Space
  uses points, 72/inch).
- The cut-session order (`cut::CUT_FLOW`), from the observed Design Space flow:
  start session, select device, machine summary, open, material select, read
  pressure table, tool list, **path data**, load/go/progress/complete.

## What needs a decrypted full cut (VERIFY)

The draw-vector wire format in `cricut-proto::wire`:

- opcode `0x42` (DrawVectorFloat, from the static analysis),
- a count byte (assumed),
- records of `[kind: u8][x: f32][y: f32]` in inches, little-endian (assumed),
- at most 56 records per frame (documented).

To confirm the exact bytes, decrypt a capture that contains a real cut (the
first capture, `joyxtra.pcapng`, has the full draw stream; it needs its session
key), line up the decrypted `0x42` frames with a known design, and correct the
packing in `wire.rs`. The same goes for the non-draw commands (material, tool,
pressure, load/go): their opcodes are known (`cricut-proto::Opcode`), their
bodies are to be read from the decrypted cut.

## Next

Once the draw format is confirmed: wire `cut::plan_draw_frames` into a
`MachineSession` (encrypt each body, send after the open phase and key exchange),
add the material/tool/pressure/load-go commands, and do a small real cut via the
Pi.
