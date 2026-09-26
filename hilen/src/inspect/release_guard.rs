// The compile time half of the rule that a shipped app never carries the
// inspect server. `build.rs` includes this file, the lib builds it only for
// its tests. Plain comments on purpose, an inner doc comment breaks the
// include.

/// Set by every shipped build, see `build/release/with-secrets.sh` and the
/// release scripts next to it.
pub(crate) const RELEASE_ENV: &str = "HILEN_RELEASE";

/// Why the build has to stop, `None` when it may go on. An empty mark counts
/// as unset, the same as for the session key.
pub(crate) fn inspect_release_error(inspect: bool, release_mark: Option<&str>) -> Option<String> {
    let release = release_mark.is_some_and(|mark| !mark.is_empty());

    (inspect && release).then(|| {
        format!(
            r"The hilen `inspect` feature is on in a release build, {RELEASE_ENV} is set.
The inspect server lets anyone on the network read and drive the app, a shipped app must never carry it.
Turn the feature off for release builds, see docs/inspect.md in the hilen repo."
        )
    })
}

#[cfg(test)]
mod test {
    use super::inspect_release_error;

    #[test]
    fn release_with_inspect_stops() {
        assert!(inspect_release_error(true, Some("1")).is_some());
    }

    #[test]
    fn release_without_inspect_goes_on() {
        assert!(inspect_release_error(false, Some("1")).is_none());
    }

    #[test]
    fn dev_build_with_inspect_goes_on() {
        assert!(inspect_release_error(true, None).is_none());
        assert!(inspect_release_error(true, Some("")).is_none());
    }
}
