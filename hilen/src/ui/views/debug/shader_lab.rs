use std::ops::DerefMut;

use ui_proc::view;

use crate::{
    deps::refs::{Own, Weak},
    gm::{
        LossyConvert,
        color::{BLACK, Color, WHITE},
        flat::{CornerRadii, Rect, Size},
    },
    render::{
        InstanceBinding, LabPipeline,
        data::{RectView, UIRectInstance},
    },
    ui::{Label, UIManager, ViewCallbacks, ViewData, ViewFrame, ViewSubviews},
    window::{RenderPass, image::Image},
};

/// One shader to compare against the others. The source must be a drop in
/// replacement for `ui_rect.wgsl`, see `LabPipeline`.
pub struct ShaderVariant {
    pub name:      String,
    pub source:    String,
    /// Take the model vertex from a `Vertex2D` buffer, position plus uv, the
    /// way `ui_image.wgsl` does, instead of a bare `Point`.
    pub vertex_uv: bool,
    /// Bind a texture and a sampler at group 2, the way `ui_image.wgsl` does.
    /// A variant that leaves this off still gets the same pipeline otherwise,
    /// so the two can be compared with nothing else moving.
    pub textured:  bool,
}

impl ShaderVariant {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name:      name.into(),
            source:    source.into(),
            vertex_uv: false,
            textured:  false,
        }
    }

    pub fn vertex_uv(mut self) -> Self {
        self.vertex_uv = true;
        self
    }

    pub fn textured(mut self) -> Self {
        self.textured = true;
        self
    }
}

/// Draws a set of shaders side by side, one cell each, on the same screen and
/// the same GPU. Built for the questions a single shader cannot answer: which
/// of these looks right, which survives this device, which is cheapest.
///
/// A cell that stays empty is a shader that drew nothing, which is a result in
/// itself. A driver can accept a pipeline and then quietly draw nothing, so
/// the only honest check is looking at the pixels.
#[view]
pub struct ShaderLab {
    pipelines: Own<Vec<LabPipeline>>,
    cells:     Vec<Cell>,
    instances: usize,

    /// Bound at group 2 for every textured variant. Without it a textured
    /// variant draws nothing, which would read as a shader failure.
    image: Weak<Image>,

    /// The size the cells were built for. The view has no size during `setup`,
    /// so the grid can only be built once a layout has happened.
    laid_out: Size,
}

struct Cell {
    name:  String,
    label: Weak<Label>,
    rect:  Rect,
    color: Color,
}

impl ShaderLab {
    /// How many rects every variant draws per frame. Raise it to turn the lab
    /// into a load test and read the fps report.
    pub fn set_instances(mut self: Weak<Self>, instances: usize) -> Weak<Self> {
        self.instances = instances.max(1);
        self
    }

    /// Bound at group 2 for every textured variant.
    pub fn set_image(mut self: Weak<Self>, image: Weak<Image>) -> Weak<Self> {
        self.image = image;
        self
    }

    pub fn set_variants(mut self: Weak<Self>, variants: Vec<ShaderVariant>) -> Weak<Self> {
        self.pipelines.clear();
        self.cells.clear();
        self.remove_all_subviews();
        self.laid_out = Size::default();

        for variant in variants {
            self.pipelines.push(LabPipeline::new(
                &variant.name,
                &variant.source,
                variant.vertex_uv,
                variant.textured,
                InstanceBinding::device(),
            ));

            let label = self.add_view::<Label>();

            self.cells.push(Cell {
                name: variant.name,
                label,
                rect: Rect::default(),
                // A distinct color per cell tells a shader that renders with
                // the wrong data apart from one that renders correctly.
                color: Color::random(),
            });
        }

        self
    }

    fn build_grid(&mut self) {
        let count = self.cells.len();

        if count == 0 {
            return;
        }

        let columns: usize = if count > 6 { 2 } else { 1 };
        let rows = count.div_ceil(columns);

        let cell_width = self.width() / columns.lossy_convert();
        let cell_height = self.height() / rows.lossy_convert();

        for (index, cell) in self.cells.iter_mut().enumerate() {
            let column: f32 = (index % columns).lossy_convert();
            let row: f32 = (index / columns).lossy_convert();

            cell.rect = Rect::new(
                column * cell_width + 2.0,
                row * cell_height + 2.0,
                cell_width - 4.0,
                cell_height - 4.0,
            );

            let label = cell.label;
            label.set_text(cell.name.clone());
            label.set_text_size(14);
            label.set_text_color(BLACK);
            label.set_color(WHITE);
            label.set_frame((cell.rect.origin.x, cell.rect.origin.y, 120, 18));
        }
    }
}

impl ViewCallbacks for ShaderLab {
    fn update(&mut self) {
        if self.size() == self.laid_out {
            return;
        }

        self.laid_out = self.size();
        self.build_grid();
    }

    fn before_render(&self, pass: &mut RenderPass) {
        let resolution = UIManager::render_area();
        let scale = UIManager::scale();
        let instances = self.instances.max(1);

        let mut pipelines = self.pipelines.weak();

        for (index, cell) in self.cells.iter().enumerate() {
            let pipeline = &mut pipelines.deref_mut()[index];

            for _ in 0..instances {
                pipeline.add(UIRectInstance::new(
                    cell.rect,
                    cell.color,
                    BLACK,
                    0.0,
                    CornerRadii::default(),
                    0.1,
                    scale,
                ));
            }

            let image = if self.image.is_ok() {
                Some(self.image.bind())
            } else {
                None
            };

            pipeline.draw(
                pass,
                RectView {
                    resolution,
                    _padding: 0,
                },
                image,
            );
        }
    }
}
