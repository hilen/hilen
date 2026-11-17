# Formula plot

The `plot` feature reads formulas typed as text and draws them, `hilen::ui::{graphs,
Graph, GraphView}` and `hilen::ui::formula`. It came from the math app nodeline, banda
draws the `plot` blocks of an agent with it. The code is
`hilen/src/ui/views/complex/formula_plot/`.

- `y = sin(x)` is a curve, `z = x*y` a surface and `w = 1/z` a complex function, drawn as
  a surface of its size colored by its angle. `formula::Expression` parses 1 line with
  [exmex](https://github.com/bertiqwerty/exmex), the complex functions run on
  [num-complex](https://github.com/rust-num/num-complex).
- `graphs(block)` reads a block, 1 formula per line. Every curve of a block goes into 1
  graph, each surface and complex function has its own, and a line `x = a..b` sets the
  range every graph starts at. A line that does not read stands as its error.
- `GraphView::set_graph(graph, live)` draws 1 graph. Live, a drag pans or turns it and
  the wheel zooms. Still, for a list or a chat, it takes no input and a 3D graph is a
  picture, `SurfacePlotView::picture`.

A curve needs only `plot`, on `PlotView`. A surface and a complex function need `scene`
too, the 3D part is `graph_view_3d.rs`. Without `scene`, `graphs` answers with an error
for such a line, so an app shows it in that slot like any other line that does not read.

## Tests

`Graph view look` draws 2 curves of 1 block in the light and the dark theme. The probes
on the flat tops of the curves are left out, 1 sample draws a row there all curve or all
ground. The parser and the reading of a block have unit tests next to them.
