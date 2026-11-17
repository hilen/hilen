//! A tree of math nodes as positioned glyphs and rules, the way TeX sets
//! a formula, with every distance taken from the MATH table of the font.
//! All values are in points, y goes up from the baseline.

use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use rustybuzz::ttf_parser::{
    Face, GlyphId,
    math::{Constants, GlyphConstruction, MathValue, Table},
};

use super::parse::{Class, GridKind, Level, Node};
use crate::{gm::LossyConvert, text::MATH_FONT_DATA};

/// A glyph or a filled rect of a laid out formula.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    /// A glyph with its origin on its baseline at `x`, `y`, drawn at
    /// `size` points per em. `ch` is the char it stands for, `None` for a
    /// bigger variant of a glyph that has no char of its own.
    Glyph {
        id:   u16,
        ch:   Option<char>,
        x:    f32,
        y:    f32,
        size: f32,
    },
    /// A rect with its bottom left corner at `x`, `y`.
    Rule {
        x:      f32,
        y:      f32,
        width:  f32,
        height: f32,
    },
}

/// A laid out piece of a formula. `ascent` is how far it reaches above
/// its baseline, `depth` how far below, both positive.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MathBox {
    pub width:  f32,
    pub ascent: f32,
    pub depth:  f32,
    pub items:  Vec<Item>,
    /// The italic correction of the glyph this box is, 0 for any other.
    italic:     f32,
    /// The top accent attachment of the glyph this box is.
    accent_x:   Option<f32>,
    /// The box is 1 glyph, the base of a script uses other rules then.
    is_glyph:   bool,
}

impl MathBox {
    fn space(width: f32) -> Self {
        Self {
            width,
            ..Self::default()
        }
    }

    /// Puts `other` into this box with its origin at `x`, `y`.
    fn add(&mut self, other: MathBox, x: f32, y: f32) {
        self.ascent = self.ascent.max(other.ascent + y);
        self.depth = self.depth.max(other.depth - y);
        self.items.extend(other.items.into_iter().map(|item| match item {
            Item::Glyph {
                id,
                ch,
                x: gx,
                y: gy,
                size,
            } => Item::Glyph {
                id,
                ch,
                x: gx + x,
                y: gy + y,
                size,
            },
            Item::Rule {
                x: rx,
                y: ry,
                width,
                height,
            } => Item::Rule {
                x: rx + x,
                y: ry + y,
                width,
                height,
            },
        }));
    }

    fn rule(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.items.push(Item::Rule { x, y, width, height });
        self.ascent = self.ascent.max(y + height);
        self.depth = self.depth.max(-y);
    }

    /// Moves the content up by `dy`.
    fn raised(mut self, dy: f32) -> MathBox {
        for item in &mut self.items {
            match item {
                Item::Glyph { y, .. } | Item::Rule { y, .. } => *y += dy,
            }
        }
        self.ascent += dy;
        self.depth -= dy;
        self
    }
}

/// The math font, parsed once.
struct MathFont {
    face:  Face<'static>,
    table: Table<'static>,
    upem:  f32,
}

fn math_font() -> Result<&'static MathFont> {
    static FONT: OnceLock<Option<MathFont>> = OnceLock::new();
    FONT.get_or_init(|| {
        let face = Face::parse(MATH_FONT_DATA, 0).ok()?;
        let table = face.tables().math?;
        table.constants?;
        Some(MathFont {
            upem: f32::from(face.units_per_em()),
            face,
            table,
        })
    })
    .as_ref()
    .ok_or_else(|| anyhow!("the math font has no MATH table"))
}

/// The style of a part of a formula: its size and whether it is cramped,
/// set lower, like a denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Style {
    level:   Level,
    cramped: bool,
}

impl Style {
    fn sup(self) -> Self {
        Self {
            level:   match self.level {
                Level::Display | Level::Text => Level::Script,
                Level::Script | Level::ScriptScript => Level::ScriptScript,
            },
            cramped: self.cramped,
        }
    }

    fn sub(self) -> Self {
        Self {
            cramped: true,
            ..self.sup()
        }
    }

    fn num(self) -> Self {
        Self {
            level:   match self.level {
                Level::Display => Level::Text,
                Level::Text => Level::Script,
                Level::Script | Level::ScriptScript => Level::ScriptScript,
            },
            cramped: self.cramped,
        }
    }

    fn den(self) -> Self {
        Self {
            cramped: true,
            ..self.num()
        }
    }

    fn cramped(self) -> Self {
        Self {
            cramped: true,
            ..self
        }
    }

    fn at(self, level: Level) -> Self {
        Self { level, ..self }
    }

    fn is_display(self) -> bool {
        self.level == Level::Display
    }

    fn is_script(self) -> bool {
        self.level >= Level::Script
    }
}

/// Lays `nodes` out at `size` points per em, as a display formula or as
/// one inside a line.
pub(crate) fn lay_out(nodes: &[Node], size: f32, display: bool) -> Result<MathBox> {
    let font = math_font()?;
    let layout = Layout { font, size };
    let style = Style {
        level:   if display { Level::Display } else { Level::Text },
        cramped: false,
    };
    Ok(layout.list(nodes, style))
}

/// An atom of a list before the space between atoms goes in.
enum Atom {
    Box { class: Class, body: MathBox },
    Glue(f32),
}

struct Layout {
    font: &'static MathFont,
    /// Points per em of the text style.
    size: f32,
}

impl Layout {
    fn constants(&self) -> Constants<'static> {
        self.font.table.constants.expect("checked when the font was parsed")
    }

    /// Points per em of a style.
    fn em(&self, style: Style) -> f32 {
        let constants = self.constants();
        let percent = match style.level {
            Level::Display | Level::Text => 100,
            Level::Script => constants.script_percent_scale_down(),
            Level::ScriptScript => constants.script_script_percent_scale_down(),
        };
        self.size * f32::from(percent) / 100.0
    }

    /// Points per font unit in a style.
    fn unit(&self, style: Style) -> f32 {
        self.em(style) / self.font.upem
    }

    fn value(&self, value: MathValue, style: Style) -> f32 {
        f32::from(value.value) * self.unit(style)
    }

    fn glyph(&self, ch: char, style: Style) -> MathBox {
        if let Some(id) = self.font.face.glyph_index(ch) {
            return self.glyph_box(id, Some(ch), style);
        }
        // A char the math font lacks still draws, through the fallback
        // fonts of the label, so it gets a guessed box of 1 letter.
        let em = self.em(style);
        MathBox {
            width: em * 0.6,
            ascent: em * 0.7,
            depth: em * 0.2,
            items: vec![Item::Glyph {
                id:   0,
                ch:   Some(ch),
                x:    0.0,
                y:    0.0,
                size: em,
            }],
            is_glyph: true,
            ..MathBox::default()
        }
    }

    fn glyph_box(&self, id: GlyphId, ch: Option<char>, style: Style) -> MathBox {
        let face = &self.font.face;
        let unit = self.unit(style);
        let advance = f32::from(face.glyph_hor_advance(id).unwrap_or_default()) * unit;
        let (ascent, depth) = face.glyph_bounding_box(id).map_or((0.0, 0.0), |rect| {
            (f32::from(rect.y_max) * unit, -f32::from(rect.y_min) * unit)
        });
        let info = self.font.table.glyph_info;
        let italic = info
            .and_then(|info| info.italic_corrections)
            .and_then(|values| values.get(id))
            .map_or(0.0, |value| self.value(value, style));
        let accent_x = info
            .and_then(|info| info.top_accent_attachments)
            .and_then(|values| values.get(id))
            .map(|value| self.value(value, style));
        MathBox {
            width: advance,
            ascent,
            depth,
            items: vec![Item::Glyph {
                id: id.0,
                ch,
                x: 0.0,
                y: 0.0,
                size: self.em(style),
            }],
            italic,
            accent_x,
            is_glyph: true,
        }
    }

    fn construction(&self, ch: char, vertical: bool) -> Option<(GlyphId, GlyphConstruction<'static>)> {
        let id = self.font.face.glyph_index(ch)?;
        let variants = self.font.table.variants?;
        let constructions = if vertical {
            variants.vertical_constructions
        } else {
            variants.horizontal_constructions
        };
        Some((id, constructions.get(id)?))
    }

    /// The glyph of `ch` at least `target` points tall, or wide when not
    /// `vertical`: a bigger variant of the font, or one built of parts.
    fn stretched(&self, ch: char, target: f32, vertical: bool, style: Style) -> MathBox {
        let Some((id, construction)) = self.construction(ch, vertical) else {
            return self.glyph(ch, style);
        };
        let unit = self.unit(style);
        let mut biggest = None;
        for variant in construction.variants {
            biggest = Some(variant.variant_glyph);
            if f32::from(variant.advance_measurement) * unit >= target {
                return self.variant(variant.variant_glyph, id, ch, style);
            }
        }
        if let Some(assembly) = construction.assembly {
            return self.assembled(assembly.parts.into_iter().collect(), target, vertical, style);
        }
        match biggest {
            Some(glyph) => self.variant(glyph, id, ch, style),
            None => self.glyph(ch, style),
        }
    }

    fn variant(&self, glyph: GlyphId, base: GlyphId, ch: char, style: Style) -> MathBox {
        self.glyph_box(glyph, (glyph == base).then_some(ch), style)
    }

    /// The parts of a glyph assembly put end to end, with as many copies
    /// of the parts that repeat as it takes to reach `target`.
    fn assembled(
        &self,
        parts: Vec<rustybuzz::ttf_parser::math::GlyphPart>,
        target: f32,
        vertical: bool,
        style: Style,
    ) -> MathBox {
        let unit = self.unit(style);
        let min_overlap = self
            .font
            .table
            .variants
            .map_or(0.0, |variants| f32::from(variants.min_connector_overlap) * unit);
        let advance = |part: &rustybuzz::ttf_parser::math::GlyphPart| f32::from(part.full_advance) * unit;

        // The fewest repeats whose length, with the least overlap, reaches
        // the target. 100 is far more than any formula asks for.
        let mut chosen = Vec::new();
        for repeats in 0..100 {
            chosen = parts
                .iter()
                .flat_map(|part| {
                    let count = if part.part_flags.extender() { repeats } else { 1 };
                    std::iter::repeat_n(*part, count)
                })
                .collect::<Vec<_>>();
            let joints: f32 = chosen.len().saturating_sub(1).lossy_convert();
            let longest: f32 = chosen.iter().map(advance).sum::<f32>() - joints * min_overlap;
            if longest >= target {
                break;
            }
        }
        let joints = chosen.len().saturating_sub(1);
        let joint_count: f32 = joints.lossy_convert();
        let longest: f32 = chosen.iter().map(advance).sum::<f32>() - joint_count * min_overlap;
        // The spare length goes into the joints, each joint at most as
        // long as its shorter connector.
        let spare = if joints == 0 {
            0.0
        } else {
            (longest - target).max(0.0) / joint_count
        };

        let mut result = MathBox::default();
        let mut at = 0.0;
        for (index, part) in chosen.iter().enumerate() {
            let piece = self.glyph_box(part.glyph_id, None, style);
            if vertical {
                let bottom = self
                    .font
                    .face
                    .glyph_bounding_box(part.glyph_id)
                    .map_or(0.0, |rect| f32::from(rect.y_min) * unit);
                result.width = result.width.max(piece.width);
                result.add(piece, 0.0, at - bottom);
            } else {
                result.add(piece, at, 0.0);
            }
            if let Some(next) = chosen.get(index + 1) {
                let most = f32::from(part.end_connector_length.min(next.start_connector_length)) * unit;
                let overlap = (min_overlap + spare).min(most.max(min_overlap));
                at += advance(part) - overlap;
            } else {
                at += advance(part);
            }
        }
        if vertical {
            result.ascent = at;
            result.depth = 0.0;
        } else {
            result.width = at;
        }
        result
    }

    /// A stretched bracket centered on the math axis.
    fn delimiter(&self, ch: char, target: f32, style: Style) -> MathBox {
        let glyph = self.stretched(ch, target, true, style);
        let axis = self.value(self.constants().axis_height(), style);
        let dy = axis - (glyph.ascent - glyph.depth) / 2.0;
        glyph.raised(dy)
    }

    fn list(&self, nodes: &[Node], style: Style) -> MathBox {
        let mut style = style;
        let mut atoms: Vec<Atom> = Vec::new();
        for node in nodes {
            match node {
                Node::Style(level) => style = style.at(*level),
                Node::Space(em) => atoms.push(Atom::Glue(em * self.em(style))),
                _ => {
                    let (class, body) = self.node(node, style);
                    atoms.push(Atom::Box { class, body });
                }
            }
        }

        // A binary operator with nothing to join is an ordinary atom, the
        // minus of `-1` or of `(-x)`.
        let mut previous: Option<usize> = None;
        for index in 0..atoms.len() {
            let Atom::Box { class, .. } = atoms[index] else {
                continue;
            };
            let before = previous.map(|at| match &atoms[at] {
                Atom::Box { class, .. } => *class,
                Atom::Glue(_) => Class::Ord,
            });
            if class == Class::Bin
                && matches!(
                    before,
                    None | Some(Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct)
                )
            {
                set_class(&mut atoms[index], Class::Ord);
            }
            if matches!(class, Class::Rel | Class::Close | Class::Punct)
                && let Some(at) = previous
                && matches!(
                    atoms[at],
                    Atom::Box {
                        class: Class::Bin,
                        ..
                    }
                )
            {
                set_class(&mut atoms[at], Class::Ord);
            }
            previous = Some(index);
        }
        if let Some(at) = previous
            && matches!(
                atoms[at],
                Atom::Box {
                    class: Class::Bin,
                    ..
                }
            )
        {
            set_class(&mut atoms[at], Class::Ord);
        }

        let mut result = MathBox::default();
        let mut x = 0.0;
        let mut last: Option<Class> = None;
        let count = atoms.len();
        for atom in atoms {
            match atom {
                Atom::Glue(width) => x += width,
                Atom::Box { class, body } => {
                    if let Some(left) = last {
                        x += spacing(left, class, style) * self.em(style);
                    }
                    let width = body.width;
                    let italic = body.italic;
                    let single = count == 1;
                    if single {
                        result.italic = body.italic;
                        result.accent_x = body.accent_x;
                        result.is_glyph = body.is_glyph;
                    }
                    result.add(body, x, 0.0);
                    x += width;
                    // An italic letter leans past its advance, TeX adds the
                    // lean after it so the next atom does not touch it.
                    if !single {
                        x += italic;
                    }
                    last = Some(class);
                }
            }
        }
        result.width = x;
        result
    }

    /// The class of a node and its box.
    fn node(&self, node: &Node, style: Style) -> (Class, MathBox) {
        match node {
            Node::Char { ch, class } => (*class, self.glyph(*ch, style)),
            Node::Text(text) => (Class::Ord, self.text(text, style)),
            Node::Group(list) => {
                let mut body = self.list(list, style);
                // A group is 1 atom, its lean is not the one of a glyph.
                body.is_glyph = false;
                (Class::Ord, body)
            }
            Node::Scripts { base, sup, sub } => self.scripts(base, sup.as_deref(), sub.as_deref(), style),
            Node::Frac {
                num,
                den,
                rule,
                level,
                delims,
            } => (
                Class::Inner,
                self.fraction(num, den, *rule, *level, *delims, style),
            ),
            Node::Sqrt { index, body } => (Class::Ord, self.root(index.as_deref(), body, style)),
            Node::Op { .. } => (Class::Op, self.operator(node, style)),
            Node::Fenced { left, right, body } => (Class::Inner, self.fenced(*left, *right, body, style)),
            Node::Big { ch, size, class } => {
                let em = self.em(style);
                let target = [1.2, 1.8, 2.4, 3.0][usize::from(size.saturating_sub(1).min(3))] * em;
                (*class, self.delimiter(*ch, target, style))
            }
            Node::Accent { ch, wide, body } => (Class::Ord, self.accent(*ch, *wide, body, style)),
            Node::Overline(body) => (Class::Ord, self.overline(body, style)),
            Node::Underline(body) => (Class::Ord, self.underline(body, style)),
            Node::Brace { over, body } => (Class::Ord, self.brace(*over, body, style)),
            Node::Stack { base, over, under } => {
                let class = match base.as_slice() {
                    [Node::Char { class, .. }] => *class,
                    _ => Class::Ord,
                };
                let base = self.list(base, style);
                (class, self.limits(base, over.as_deref(), under.as_deref(), style))
            }
            Node::Grid { kind, rows } => (Class::Inner, self.grid(*kind, rows, style)),
            Node::Space(em) => (Class::Ord, MathBox::space(em * self.em(style))),
            Node::Style(_) => (Class::Ord, MathBox::default()),
        }
    }

    /// Upright text, every char after the other with no math spacing.
    fn text(&self, text: &str, style: Style) -> MathBox {
        let mut result = MathBox::default();
        let mut x = 0.0;
        for ch in text.chars() {
            if ch == ' ' {
                x += self.space_width(style);
                continue;
            }
            let glyph = self.glyph(ch, style);
            let width = glyph.width;
            result.add(glyph, x, 0.0);
            x += width;
        }
        result.width = x;
        result
    }

    fn space_width(&self, style: Style) -> f32 {
        self.font
            .face
            .glyph_index(' ')
            .and_then(|id| self.font.face.glyph_hor_advance(id))
            .map_or(self.em(style) / 3.0, |advance| {
                f32::from(advance) * self.unit(style)
            })
    }

    fn scripts(
        &self,
        base: &Node,
        sup: Option<&[Node]>,
        sub: Option<&[Node]>,
        style: Style,
    ) -> (Class, MathBox) {
        // An operator with limits, or a brace, takes its scripts above and
        // below.
        match base {
            Node::Op {
                limits,
                display_limits,
                ..
            } if limits.unwrap_or(*display_limits && style.is_display()) => {
                let op = self.operator(base, style);
                return (Class::Op, self.limits(op, sup, sub, style));
            }
            Node::Brace { over, body } => {
                let brace = self.brace(*over, body, style);
                let (sup, sub) = if *over { (sup, None) } else { (None, sub) };
                return (Class::Ord, self.limits(brace, sup, sub, style));
            }
            _ => {}
        }
        let (class, base_box) = self.node(base, style);
        let is_op = matches!(base, Node::Op { .. });
        (class, self.attach_scripts(base_box, is_op, sup, sub, style))
    }

    /// Scripts at the right of their base, the rules of the MATH table.
    fn attach_scripts(
        &self,
        base: MathBox,
        is_op: bool,
        sup: Option<&[Node]>,
        sub: Option<&[Node]>,
        style: Style,
    ) -> MathBox {
        let c = self.constants();
        let raised = style.sup();
        let lowered = style.sub();
        let sup = sup.map(|nodes| self.list(nodes, raised));
        let sub = sub.map(|nodes| self.list(nodes, lowered));

        // A glyph sits on the baseline, a bigger box drops its scripts
        // from its own top and bottom.
        let plain = base.is_glyph && !is_op;
        let mut up = sup.as_ref().map_or(0.0, |sup| {
            let shift = if style.cramped {
                self.value(c.superscript_shift_up_cramped(), style)
            } else {
                self.value(c.superscript_shift_up(), style)
            };
            let drop = if plain {
                0.0
            } else {
                base.ascent - self.value(c.superscript_baseline_drop_max(), raised)
            };
            let bottom = sup.depth + self.value(c.superscript_bottom_min(), style);
            shift.max(drop).max(bottom)
        });
        let mut down = sub.as_ref().map_or(0.0, |sub| {
            let shift = self.value(c.subscript_shift_down(), style);
            let drop = if plain {
                0.0
            } else {
                base.depth + self.value(c.subscript_baseline_drop_min(), lowered)
            };
            let top = sub.ascent - self.value(c.subscript_top_max(), style);
            shift.max(drop).max(top)
        });
        if let (Some(sup_box), Some(sub_box)) = (&sup, &sub) {
            let gap = (up - sup_box.depth) - (sub_box.ascent - down);
            let min_gap = self.value(c.sub_superscript_gap_min(), style);
            if gap < min_gap {
                down += min_gap - gap;
                // Some of it goes up, as long as the bottom of the
                // superscript stays under its limit.
                let most =
                    self.value(c.superscript_bottom_max_with_subscript(), style) - (up - sup_box.depth);
                if most > 0.0 {
                    let lift = most.min(min_gap - gap);
                    up += lift;
                    down -= lift;
                }
            }
        }

        let after = self.value(c.space_after_script(), style);
        let width = base.width;
        let italic = base.italic;
        let mut result = MathBox::default();
        result.add(base, 0.0, 0.0);
        let mut right = width;
        if let Some(sup) = sup {
            // An operator leans its superscript out by its italic
            // correction, any other base adds the lean to the advance.
            let x = if is_op { width } else { width + italic };
            right = right.max(x + sup.width);
            result.add(sup, x, up);
        }
        if let Some(sub) = sub {
            let x = if is_op { width - italic } else { width };
            right = right.max(x + sub.width);
            result.add(sub, x, -down);
        }
        result.width = right + after;
        result
    }

    /// A base with formulas centered over and under it, the limits of
    /// `\sum` in a display formula.
    fn limits(&self, base: MathBox, over: Option<&[Node]>, under: Option<&[Node]>, style: Style) -> MathBox {
        let c = self.constants();
        let over = over.map(|nodes| self.list(nodes, style.sup()));
        let under = under.map(|nodes| self.list(nodes, style.sub()));
        let width = base
            .width
            .max(over.as_ref().map_or(0.0, |over| over.width))
            .max(under.as_ref().map_or(0.0, |under| under.width));
        let italic = base.italic;
        let (ascent, depth) = (base.ascent, base.depth);
        let mut result = MathBox::default();
        let base_width = base.width;
        result.add(base, (width - base_width) / 2.0, 0.0);
        if let Some(over) = over {
            let gap = (self.value(c.upper_limit_gap_min(), style) + over.depth)
                .max(self.value(c.upper_limit_baseline_rise_min(), style));
            let x = (width - over.width) / 2.0 + italic / 2.0;
            result.add(over, x, ascent + gap);
        }
        if let Some(under) = under {
            let gap = (self.value(c.lower_limit_gap_min(), style) + under.ascent)
                .max(self.value(c.lower_limit_baseline_drop_min(), style));
            let x = (width - under.width) / 2.0 - italic / 2.0;
            result.add(under, x, -(depth + gap));
        }
        result.width = width;
        result
    }

    fn operator(&self, node: &Node, style: Style) -> MathBox {
        let Node::Op { symbol, name, .. } = node else {
            return MathBox::default();
        };
        if let Some(name) = name {
            let mut text = self.text(name, style);
            text.is_glyph = false;
            return text;
        }
        let Some(ch) = *symbol else {
            return MathBox::default();
        };
        let c = self.constants();
        let glyph = if style.is_display() {
            let target = f32::from(c.display_operator_min_height()) * self.unit(style);
            self.stretched(ch, target, true, style)
        } else {
            self.glyph(ch, style)
        };
        let italic = glyph.italic;
        let axis = self.value(c.axis_height(), style);
        let dy = axis - (glyph.ascent - glyph.depth) / 2.0;
        let mut centered = glyph.raised(dy);
        centered.italic = italic;
        centered.is_glyph = false;
        centered
    }

    fn fraction(
        &self,
        num: &[Node],
        den: &[Node],
        rule: bool,
        level: Option<Level>,
        delims: Option<(char, char)>,
        style: Style,
    ) -> MathBox {
        let c = self.constants();
        let style = level.map_or(style, |level| style.at(level));
        let num = self.list(num, style.num());
        let den = self.list(den, style.den());
        let axis = self.value(c.axis_height(), style);
        let display = style.is_display();
        let thickness = if rule {
            self.value(c.fraction_rule_thickness(), style)
        } else {
            0.0
        };

        let (mut up, mut down) = if !rule {
            if display {
                (
                    self.value(c.stack_top_display_style_shift_up(), style),
                    self.value(c.stack_bottom_display_style_shift_down(), style),
                )
            } else {
                (
                    self.value(c.stack_top_shift_up(), style),
                    self.value(c.stack_bottom_shift_down(), style),
                )
            }
        } else if display {
            (
                self.value(c.fraction_numerator_display_style_shift_up(), style),
                self.value(c.fraction_denominator_display_style_shift_down(), style),
            )
        } else {
            (
                self.value(c.fraction_numerator_shift_up(), style),
                self.value(c.fraction_denominator_shift_down(), style),
            )
        };
        if rule {
            let (num_gap, den_gap) = if display {
                (
                    self.value(c.fraction_num_display_style_gap_min(), style),
                    self.value(c.fraction_denom_display_style_gap_min(), style),
                )
            } else {
                (
                    self.value(c.fraction_numerator_gap_min(), style),
                    self.value(c.fraction_denominator_gap_min(), style),
                )
            };
            up = up.max(axis + thickness / 2.0 + num_gap + num.depth);
            down = down.max(den.ascent - (axis - thickness / 2.0) + den_gap);
        } else {
            let gap = if display {
                self.value(c.stack_display_style_gap_min(), style)
            } else {
                self.value(c.stack_gap_min(), style)
            };
            let have = (up - num.depth) - (den.ascent - down);
            if have < gap {
                up += (gap - have) / 2.0;
                down += (gap - have) / 2.0;
            }
        }

        // TeX keeps a little room at both ends of the bar.
        let pad = 0.12 * self.em(style);
        let inner = num.width.max(den.width);
        let width = inner + pad * 2.0;
        let mut result = MathBox::default();
        let num_x = (width - num.width) / 2.0;
        let den_x = (width - den.width) / 2.0;
        result.add(num, num_x, up);
        result.add(den, den_x, -down);
        if rule {
            result.rule(pad, axis - thickness / 2.0, inner, thickness);
        }
        result.width = width;

        match delims {
            Some((left, right)) => self.wrap(Some(left), Some(right), result, style),
            None => result,
        }
    }

    /// The height a bracket needs to cover `body`, the rule of TeX.
    fn cover(&self, body: &MathBox, style: Style) -> f32 {
        let axis = self.value(self.constants().axis_height(), style);
        let half = (body.ascent - axis).max(body.depth + axis);
        let em = self.em(style);
        (half * 2.0 * 0.901).max(half * 2.0 - 0.5 * em)
    }

    /// `body` between 2 brackets that grow to its height.
    fn wrap(&self, left: Option<char>, right: Option<char>, body: MathBox, style: Style) -> MathBox {
        let target = self.cover(&body, style);
        let mut result = MathBox::default();
        let mut x = 0.0;
        let null = 0.12 * self.em(style);
        match left {
            Some(ch) => {
                let glyph = self.delimiter(ch, target, style);
                let width = glyph.width;
                result.add(glyph, x, 0.0);
                x += width;
            }
            None => x += null,
        }
        let width = body.width;
        result.add(body, x, 0.0);
        x += width;
        match right {
            Some(ch) => {
                let glyph = self.delimiter(ch, target, style);
                let width = glyph.width;
                result.add(glyph, x, 0.0);
                x += width;
            }
            None => x += null,
        }
        result.width = x;
        result
    }

    fn fenced(&self, left: Option<char>, right: Option<char>, body: &[Node], style: Style) -> MathBox {
        let body = self.list(body, style);
        self.wrap(left, right, body, style)
    }

    fn root(&self, index: Option<&[Node]>, body: &[Node], style: Style) -> MathBox {
        let c = self.constants();
        let body = self.list(body, style.cramped());
        let mut gap = if style.is_display() {
            self.value(c.radical_display_style_vertical_gap(), style)
        } else {
            self.value(c.radical_vertical_gap(), style)
        };
        let thickness = self.value(c.radical_rule_thickness(), style);
        let extra = self.value(c.radical_extra_ascender(), style);
        let target = body.ascent + body.depth + gap + thickness;
        let sign = self.stretched('√', target, true, style);
        let total = sign.ascent + sign.depth;
        if total > target {
            gap += (total - target) / 2.0;
        }
        let top = body.ascent + gap + thickness;
        let dy = top - sign.ascent;
        let sign = sign.raised(dy);
        let sign_width = sign.width;
        let sign_bottom = -sign.depth;

        let mut result = MathBox::default();
        let mut x = 0.0;
        if let Some(index) = index {
            let index = self.list(index, style.at(Level::ScriptScript));
            let before = self.value(c.radical_kern_before_degree(), style);
            let after = self.value(c.radical_kern_after_degree(), style);
            let raise = f32::from(c.radical_degree_bottom_raise_percent()) / 100.0 * total;
            let index_width = index.width;
            let index_depth = index.depth;
            result.add(index, before, sign_bottom + raise + index_depth);
            x = (before + index_width + after).max(0.0);
        }
        result.add(sign, x, 0.0);
        x += sign_width;
        let body_width = body.width;
        result.add(body, x, 0.0);
        result.rule(x, top - thickness, body_width, thickness);
        result.ascent = result.ascent.max(top + extra);
        result.width = x + body_width;
        result
    }

    fn accent(&self, ch: char, wide: bool, body: &[Node], style: Style) -> MathBox {
        let c = self.constants();
        let base = self.list(body, style.cramped());
        let accent = if wide {
            self.stretched(ch, base.width, false, style)
        } else {
            self.glyph(ch, style)
        };
        let base_height = self.value(c.accent_base_height(), style);
        let lift = (base.ascent - base_height).max(0.0);
        // The accent is centered on the attachment point of the base, the
        // middle of a letter as the font designer put it.
        let attach = base.accent_x.unwrap_or(base.width / 2.0);
        let accent_center = accent.accent_x.unwrap_or_else(|| ink_center(self, &accent));
        let width = base.width;
        let mut result = MathBox {
            italic: base.italic,
            ..MathBox::default()
        };
        result.add(base, 0.0, 0.0);
        result.add(accent, attach - accent_center, lift);
        result.width = width;
        result
    }

    fn overline(&self, body: &[Node], style: Style) -> MathBox {
        let c = self.constants();
        let body = self.list(body, style.cramped());
        let gap = self.value(c.overbar_vertical_gap(), style);
        let thickness = self.value(c.overbar_rule_thickness(), style);
        let extra = self.value(c.overbar_extra_ascender(), style);
        let (width, ascent) = (body.width, body.ascent);
        let mut result = MathBox::default();
        result.add(body, 0.0, 0.0);
        result.rule(0.0, ascent + gap, width, thickness);
        result.ascent += extra;
        result.width = width;
        result
    }

    fn underline(&self, body: &[Node], style: Style) -> MathBox {
        let c = self.constants();
        let body = self.list(body, style);
        let gap = self.value(c.underbar_vertical_gap(), style);
        let thickness = self.value(c.underbar_rule_thickness(), style);
        let extra = self.value(c.underbar_extra_descender(), style);
        let (width, depth) = (body.width, body.depth);
        let mut result = MathBox::default();
        result.add(body, 0.0, 0.0);
        result.rule(0.0, -(depth + gap + thickness), width, thickness);
        result.depth += extra;
        result.width = width;
        result
    }

    fn brace(&self, over: bool, body: &[Node], style: Style) -> MathBox {
        let c = self.constants();
        let body = self.list(body, style);
        let ch = if over { '⏞' } else { '⏟' };
        let brace = self.stretched(ch, body.width, false, style);
        let gap = self.value(c.stretch_stack_gap_below_min(), style);
        let (width, ascent, depth) = (body.width, body.ascent, body.depth);
        let brace_width = brace.width;
        let x = (width - brace_width) / 2.0;
        let mut result = MathBox::default();
        result.add(body, (brace_width - width).max(0.0) / 2.0, 0.0);
        let y = if over {
            ascent + gap + brace.depth
        } else {
            -(depth + gap + brace.ascent)
        };
        result.add(brace, x.max(0.0), y);
        result.width = width.max(brace_width);
        result
    }

    fn grid(&self, kind: GridKind, rows: &[Vec<Vec<Node>>], style: Style) -> MathBox {
        let cell_style = match kind {
            GridKind::Aligned => style.at(Level::Display),
            GridKind::Matrix(..) | GridKind::Cases if style.is_display() => style.at(Level::Text),
            GridKind::Matrix(..) | GridKind::Cases => style,
        };
        let em = self.em(style);
        let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
        let cells: Vec<Vec<MathBox>> = rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(column, cell)| {
                        // The right side of an aligned pair starts with an
                        // empty atom, so `&= b` spaces the `=` like `a = b`.
                        if kind == GridKind::Aligned && column % 2 == 1 {
                            let mut padded = vec![Node::Group(Vec::new())];
                            padded.extend(cell.iter().cloned());
                            self.list(&padded, cell_style)
                        } else {
                            self.list(cell, cell_style)
                        }
                    })
                    .collect()
            })
            .collect();
        let mut widths = vec![0.0_f32; columns];
        for row in &cells {
            for (width, cell) in widths.iter_mut().zip(row) {
                *width = width.max(cell.width);
            }
        }
        let gap_after = |column: usize| -> f32 {
            match kind {
                GridKind::Matrix(..) | GridKind::Cases => em,
                GridKind::Aligned if column % 2 == 1 => 2.0 * em,
                GridKind::Aligned => 0.0,
            }
        };

        // The baselines are at least 1.2 em apart, more when a row is tall.
        let strut_ascent = 0.85 * em;
        let strut_depth = 0.35 * em;
        let gap = match kind {
            GridKind::Aligned => 0.25 * em,
            _ => 0.2 * em,
        };
        let mut result = MathBox::default();
        let mut y = 0.0;
        for (index, row) in cells.into_iter().enumerate() {
            let ascent = row.iter().map(|cell| cell.ascent).fold(strut_ascent, f32::max);
            let depth = row.iter().map(|cell| cell.depth).fold(strut_depth, f32::max);
            if index > 0 {
                y -= ascent + gap;
            }
            let mut x = 0.0;
            for (column, cell) in row.into_iter().enumerate() {
                let room = widths[column];
                let offset = match kind {
                    GridKind::Matrix(..) => (room - cell.width) / 2.0,
                    GridKind::Aligned if column % 2 == 0 => room - cell.width,
                    GridKind::Cases | GridKind::Aligned => 0.0,
                };
                result.add(cell, x + offset, y);
                x += room + gap_after(column);
            }
            y -= depth;
        }
        let total: f32 =
            widths.iter().sum::<f32>() + (0..columns.saturating_sub(1)).map(gap_after).sum::<f32>();
        result.width = total;

        // The middle of the grid goes on the math axis.
        let axis = self.value(self.constants().axis_height(), style);
        let middle = (result.ascent - result.depth) / 2.0;
        let grid = result.raised(axis - middle);
        match kind {
            GridKind::Matrix(None, None) | GridKind::Aligned => grid,
            GridKind::Matrix(left, right) => self.wrap(left, right, padded(grid, 0.2 * em), style),
            GridKind::Cases => self.wrap(Some('{'), None, padded(grid, 0.2 * em), style),
        }
    }
}

/// `body` with room at both sides.
fn padded(body: MathBox, pad: f32) -> MathBox {
    let width = body.width + pad * 2.0;
    let mut result = MathBox::default();
    result.add(body, pad, 0.0);
    result.width = width;
    result
}

/// The middle of the ink of a 1 glyph box, for an accent with no
/// attachment point.
fn ink_center(layout: &Layout, accent: &MathBox) -> f32 {
    let Some(Item::Glyph { id, size, .. }) = accent.items.first() else {
        return accent.width / 2.0;
    };
    let unit = size / layout.font.upem;
    layout
        .font
        .face
        .glyph_bounding_box(GlyphId(*id))
        .map_or(accent.width / 2.0, |rect| {
            f32::midpoint(f32::from(rect.x_min), f32::from(rect.x_max)) * unit
        })
}

fn set_class(atom: &mut Atom, new: Class) {
    if let Atom::Box { class, .. } = atom {
        *class = new;
    }
}

/// The space between 2 atoms in ems, the table of TeX. A space in
/// parentheses there is only set in the display and the text style.
fn spacing(left: Class, right: Class, style: Style) -> f32 {
    use Class::{Bin, Close, Inner, Op, Open, Ord, Punct, Rel};
    const THIN: f32 = 3.0 / 18.0;
    const MED: f32 = 4.0 / 18.0;
    const THICK: f32 = 5.0 / 18.0;
    // The second value is set in a script too.
    let (space, always) = match (left, right) {
        (Ord | Close | Inner, Op) | (Op, Ord | Op) => (THIN, true),
        (Ord | Op | Close, Inner) | (Punct, _) | (Inner, Ord | Open | Punct | Inner) => (THIN, false),
        (Ord | Op | Close | Inner, Bin) | (Bin, Ord | Op | Open | Inner) => (MED, false),
        (Ord | Op | Close | Inner, Rel) | (Rel, Ord | Op | Open | Inner) => (THICK, false),
        _ => (0.0, true),
    };
    if always || !style.is_script() { space } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{Item, lay_out};
    use crate::ui::views::complex::math::parse::parse;

    fn laid(source: &str, display: bool) -> super::MathBox {
        lay_out(&parse(source).unwrap(), 20.0, display).unwrap()
    }

    fn glyph_ys(source: &str) -> Vec<f32> {
        laid(source, false)
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph { y, .. } => Some(*y),
                Item::Rule { .. } => None,
            })
            .collect()
    }

    #[test]
    fn a_power_is_raised_and_smaller() {
        let power = laid("x^3", false);
        let [
            Item::Glyph {
                y: 0.0, size: base, ..
            },
            Item::Glyph {
                y: up, size: script, ..
            },
        ] = power.items[..]
        else {
            panic!("2 glyphs expected, got {:?}", power.items);
        };
        assert!(up > 5.0, "the power is raised by {up}");
        assert!(script < base, "the power is {script} points, the base {base}");
    }

    #[test]
    fn a_fraction_puts_its_parts_over_and_under_a_bar() {
        let fraction = laid(r"\frac{a}{b}", true);
        let ys = glyph_ys(r"\frac{a}{b}");
        assert!(ys[0] > 0.0 && ys[1] < 0.0, "the parts are at {ys:?}");
        let bars = fraction.items.iter().filter(|item| matches!(item, Item::Rule { .. })).count();
        assert_eq!(bars, 1);
    }

    #[test]
    fn a_display_sum_is_bigger_and_has_its_limits_over_and_under() {
        let inline = laid(r"\sum_{n=1}^{N} n", false);
        let display = laid(r"\sum_{n=1}^{N} n", true);
        assert!(display.ascent + display.depth > (inline.ascent + inline.depth) * 1.3);
        assert!(
            inline.width > display.width * 0.9,
            "inline limits sit at the side"
        );
    }

    #[test]
    fn brackets_around_a_fraction_grow_to_cover_it() {
        let plain = laid(r"(x)", true);
        let grown = laid(r"\left(\frac{a}{b}\right)", true);
        assert!(grown.ascent + grown.depth > (plain.ascent + plain.depth) * 1.5);
    }

    #[test]
    fn a_root_has_a_sign_and_a_bar() {
        let root = laid(r"\sqrt{x}", false);
        assert!(root.items.iter().any(|item| matches!(item, Item::Rule { .. })));
        assert!(root.items.iter().any(|item| matches!(item, Item::Glyph { ch: Some('√'), .. })));
    }

    #[test]
    fn relations_get_more_room_than_binary_operators() {
        // a=b is wider than a+b, a thick space against a medium one.
        assert!(laid("a=b", false).width > laid("a+b", false).width);
    }
}
