use crate::{
    deps::refs::Weak,
    gm::color::Color,
    image::{Image, tint_svg},
};

pub struct UIImages;

/// The Lucide menu, the 3 lines of the button that opens a drawer, drawn in
/// black for a tint.
pub(crate) const MENU_SVG: &[u8] = include_bytes!("images/menu.svg");

/// The Lucide chevrons of the fold arrow of a tree, drawn in black for a
/// tint: right for a folded row, down for an open one.
pub(crate) const FOLDED_SVG: &[u8] = include_bytes!("images/fb_chevron_right.svg");
pub(crate) const UNFOLDED_SVG: &[u8] = include_bytes!("images/tree_chevron_down.svg");

impl UIImages {
    pub fn delete() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/delete.png"), "delete.png")
    }

    pub fn up() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/up.png"), "up.png")
    }

    /// Lucide chevrons, <https://lucide.dev>, ISC licensed.
    pub fn chevron_up() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/chevron_up.svg"), "chevron_up.svg")
    }

    pub fn chevron_down() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/chevron_down.svg"), "chevron_down.svg")
    }

    /// The down chevron in a lighter gray that reads on a dark fill.
    pub(crate) fn chevron_down_dark() -> Weak<Image> {
        Image::from_file_data(
            include_bytes!("images/chevron_down_dark.svg"),
            "chevron_down_dark.svg",
        )
    }

    /// The Lucide x, drawn in `tint`.
    pub(crate) fn x(tint: Color) -> Weak<Image> {
        tint_svg(include_bytes!("images/x.svg"), "x.svg", tint)
    }

    /// The Lucide chevron left, drawn in `tint`. The same file as the one
    /// of the file browser, so the same name.
    pub(crate) fn chevron_left(tint: Color) -> Weak<Image> {
        tint_svg(
            include_bytes!("images/fb_chevron_left.svg"),
            "fb_chevron_left.svg",
            tint,
        )
    }

    /// The Lucide chevron right, drawn in `tint`.
    pub(crate) fn chevron_right(tint: Color) -> Weak<Image> {
        tint_svg(
            include_bytes!("images/fb_chevron_right.svg"),
            "fb_chevron_right.svg",
            tint,
        )
    }

    /// The Lucide copy, drawn in `tint`.
    pub(crate) fn copy(tint: Color) -> Weak<Image> {
        tint_svg(include_bytes!("images/copy.svg"), "copy.svg", tint)
    }

    /// The Lucide arrow up, drawn in `tint`. The same file as the one of
    /// the file browser, so the same name.
    pub(crate) fn arrow_up(tint: Color) -> Weak<Image> {
        tint_svg(include_bytes!("images/fb_arrow_up.svg"), "fb_arrow_up.svg", tint)
    }

    /// The Lucide arrow down, drawn in `tint`.
    pub(crate) fn arrow_down(tint: Color) -> Weak<Image> {
        tint_svg(include_bytes!("images/arrow_down.svg"), "arrow_down.svg", tint)
    }

    pub fn down() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/down.png"), "down.png")
    }

    pub(crate) fn left() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/left.png"), "left.png")
    }

    pub fn right() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/right.png"), "right.png")
    }

    pub fn rb() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/rb.png"), "rb.png")
    }

    pub fn plus() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/plus.png"), "plus.png")
    }

    pub(crate) fn joystick() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/joystick.png"), "joystick.png")
    }

    pub(crate) fn handle() -> Weak<Image> {
        Image::from_file_data(include_bytes!("images/handle.png"), "handle.png")
    }
}
