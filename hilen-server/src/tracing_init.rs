use std::{
    backtrace::Backtrace,
    fmt,
    fs::File,
    panic::{set_hook, take_hook},
    sync::Mutex,
};

use tracing::{Subscriber, error, info, warn};
use tracing_subscriber::{
    EnvFilter, Layer,
    field::RecordFields,
    fmt::{
        FormatFields,
        format::{DefaultFields, Writer},
        layer,
    },
    layer::SubscriberExt,
    registry,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

use crate::log_file;

/// Sets up logging to stdout and to the log file of this start, see
/// `log_file`. `app_name` is the crate name of the backend, it sets the
/// debug level for that crate and names the log folder and the file.
pub fn init(app_name: &str) {
    let default = format!("info,{app_name}=debug,hilen_server=debug,tower_http=debug");
    let (file, path, problem) = match log_file::open(app_name) {
        Ok(Some((file, path))) => (Some(file), Some(path), None),
        Ok(None) => (None, None, None),
        Err(err) => (None, None, Some(err)),
    };

    registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default)))
        .with(layer())
        .with(file.map(file_layer))
        .init();
    log_panics();

    if let Some(path) = path {
        info!("log file: {}", path.display());
    }
    if let Some(err) = problem {
        warn!("no log file: {err:#}");
    }
}

/// The stdout lines without color codes. Each line is written to the file
/// at once, so a crash loses nothing.
fn file_layer<S>(file: File) -> impl Layer<S>
where S: Subscriber + for<'span> LookupSpan<'span> {
    layer()
        .with_ansi(false)
        .fmt_fields(PlainFields::default())
        .with_writer(Mutex::new(file))
}

/// The fields of a span are formatted once and stored on the span under the
/// type of the formatter. With the type of the stdout layer the file would
/// get the stored stdout text with its color codes, so the file layer has
/// its own type.
#[derive(Default)]
struct PlainFields(DefaultFields);

impl<'writer> FormatFields<'writer> for PlainFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// A backend with no terminal has only the log file, so a panic must reach
/// it. The earlier hook still runs and keeps the stderr output as it is.
fn log_panics() {
    let earlier = take_hook();
    set_hook(Box::new(move |info| {
        error!("{info}\nBacktrace: {}", Backtrace::force_capture());
        earlier(info);
    }));
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{OpenOptions, read_to_string, remove_file},
        io::sink,
        process::id,
    };

    use anyhow::Result;
    use tracing::{info, info_span, subscriber::with_default, warn};
    use tracing_subscriber::{fmt::layer, layer::SubscriberExt, registry};

    use super::file_layer;

    #[test]
    fn the_file_gets_the_lines_without_color_codes() -> Result<()> {
        let path = temp_dir().join(format!("hilen-server-log-layer-{}.log", id()));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        // The colored layer stands in for stdout. It sits first, as in
        // `init`, so it is the one that formats the span fields first.
        let subscriber = registry()
            .with(layer().with_ansi(true).with_writer(sink))
            .with(file_layer(file));

        with_default(subscriber, || {
            info!(films = 3, "library loaded");
            let span = info_span!("request", uri = "/films");
            span.in_scope(|| warn!("slow answer"));
        });

        let text = read_to_string(&path)?;
        remove_file(&path)?;

        assert!(!text.contains('\u{1b}'), "{text}");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(
            lines[0].contains("INFO") && lines[0].contains("library loaded films=3"),
            "{text}"
        );
        assert!(
            lines[1].contains("WARN") && lines[1].contains("slow answer"),
            "{text}"
        );
        assert!(lines[1].contains(r#"request{uri="/films"}"#), "{text}");
        Ok(())
    }
}
