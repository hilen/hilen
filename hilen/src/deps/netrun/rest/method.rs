use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Copy, Clone)]
pub enum Method {
    #[default]
    Get,
    Post,
    Patch,
    Delete,
}

impl Method {
    pub fn get(&self) -> bool {
        matches!(self, Self::Get)
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let st = match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        };

        write!(f, "{st}")
    }
}
