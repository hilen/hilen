/// 1 line of a log as a `LogView` shows it. Both texts may hold ANSI
/// color codes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogLine {
    /// What stands in front of the text, in a column of its own: a time,
    /// a name, a level. The view does not know what it means. Empty for a
    /// line with no prefix.
    pub prefix: String,
    /// The text of the line. It wraps beside the prefix.
    pub text:   String,
}

impl LogLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            prefix: String::new(),
            text:   text.into(),
        }
    }

    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }
}

/// The lines of a `LogView`. The app holds them, the view only reads
/// them and keeps no copy. After a change of the lines the app tells the
/// view: `lines_added`, `lines_removed` or `reload`.
pub trait LogData {
    fn number_of_lines(&self) -> usize;

    /// The line `index`. The view asks for a line when it measures it and
    /// every time the line comes on screen, so this has to be cheap.
    fn line(&self, index: usize) -> LogLine;
}
