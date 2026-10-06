use crate::{
    deps::refs::Weak,
    filesystem::{FileCategory, PlaceKind},
    gm::color::{Color, WHITE},
    ui::DynamicColor,
    window::image::{Image, tint_svg},
};

pub(super) const BACKGROUND: DynamicColor = DynamicColor::new(WHITE, Color::hex("#1E1E20"));
pub(super) const PANEL: DynamicColor = DynamicColor::new(Color::hex("#F4F4F6"), Color::hex("#27272A"));
pub(super) const FIELD: DynamicColor = DynamicColor::new(WHITE, Color::hex("#18181A"));
pub(super) const BORDER: DynamicColor = DynamicColor::new(Color::hex("#D9D9DE"), Color::hex("#3B3B40"));
pub(super) const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
pub(super) const DIM_TEXT: DynamicColor = DynamicColor::new(Color::hex("#6E6E76"), Color::hex("#9A9AA2"));
pub(super) const FAINT_TEXT: DynamicColor = DynamicColor::new(Color::hex("#B4B4BC"), Color::hex("#5C5C64"));
pub(super) const ACCENT: DynamicColor = DynamicColor::new(Color::hex("#0A6CFF"), Color::hex("#2F81F7"));
pub(super) const ON_ACCENT: DynamicColor = DynamicColor::new(WHITE, WHITE);
pub(super) const HOVER: DynamicColor = DynamicColor::new(Color::hex("#ECECF0"), Color::hex("#323236"));
pub(super) const PICKED_PLACE: DynamicColor = DynamicColor::new(Color::hex("#DCDCE3"), Color::hex("#3D3D43"));
pub(super) const FOLDER: DynamicColor = DynamicColor::new(Color::hex("#2F8BF5"), Color::hex("#5AA5FF"));
pub(super) const ERROR_TEXT: DynamicColor = DynamicColor::new(Color::hex("#D92D20"), Color::hex("#FF6B5E"));

pub(super) const TEXT_SIZE: f32 = 13.0;
pub(super) const SMALL_TEXT_SIZE: f32 = 12.0;
pub(super) const RADIUS: f32 = 6.0;

/// The icons of the file browser, Lucide, <https://lucide.dev>, ISC
/// licensed, drawn in black so a tint can color them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Icon {
    Folder,
    File,
    Text,
    Image,
    Film,
    Music,
    Archive,
    Code,
    Back,
    Forward,
    Up,
    Search,
    Eye,
    List,
    Grid,
    Home,
    Drive,
}

impl Icon {
    pub(super) fn of_category(category: FileCategory) -> Self {
        match category {
            FileCategory::Folder => Self::Folder,
            FileCategory::Image => Self::Image,
            FileCategory::Video => Self::Film,
            FileCategory::Audio => Self::Music,
            FileCategory::Archive => Self::Archive,
            FileCategory::Code => Self::Code,
            FileCategory::Document => Self::Text,
            FileCategory::Other => Self::File,
        }
    }

    pub(super) fn of_place(kind: PlaceKind) -> Self {
        match kind {
            PlaceKind::Home => Self::Home,
            PlaceKind::Drive => Self::Drive,
            PlaceKind::Folder => Self::Folder,
        }
    }

    fn data(self) -> (&'static [u8], &'static str) {
        macro_rules! icon {
            ($name:literal) => {
                (
                    include_bytes!(concat!("../../../images/fb_", $name, ".svg")).as_slice(),
                    concat!("fb_", $name, ".svg"),
                )
            };
        }

        match self {
            Self::Folder => icon!("folder"),
            Self::File => icon!("file"),
            Self::Text => icon!("file_text"),
            Self::Image => icon!("image"),
            Self::Film => icon!("film"),
            Self::Music => icon!("music"),
            Self::Archive => icon!("file_archive"),
            Self::Code => icon!("file_code"),
            Self::Back => icon!("chevron_left"),
            Self::Forward => icon!("chevron_right"),
            Self::Up => icon!("arrow_up"),
            Self::Search => icon!("search"),
            Self::Eye => icon!("eye"),
            Self::List => icon!("list"),
            Self::Grid => icon!("layout_grid"),
            Self::Home => icon!("house"),
            Self::Drive => icon!("hard_drive"),
        }
    }

    /// The icon in one color. Cached per icon and color, so asking on
    /// every reuse of a row costs a lookup.
    pub(super) fn image(self, tint: Color) -> Weak<Image> {
        let (data, name) = self.data();
        tint_svg(data, name, tint)
    }
}
