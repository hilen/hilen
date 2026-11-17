//! Commands an app answers for the tools, like a test flight that a
//! profile needs or a state to set up before a screenshot. The inspector
//! sends them with `hilen-inspect app <name> [args]`, see `docs/inspect.md`.
//! An app registers its handler in any build, a build with no inspect
//! server never calls it, so the app needs no `cfg` of its own.

use std::cell::RefCell;

type Handler = Box<dyn FnMut(&str, &[String]) -> Result<String, String>>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
}

/// Sets the handler of the app's commands, called on the main thread
/// with the name and the arguments of each. Its text is the answer the
/// inspector prints, its error fails the command. Register it on the main
/// thread, a later call replaces the handler.
pub fn register(handler: impl FnMut(&str, &[String]) -> Result<String, String> + 'static) {
    HANDLER.with_borrow_mut(|slot| *slot = Some(Box::new(handler)));
}

/// Runs command `name`, on the main thread.
#[cfg(feature = "inspect")]
pub(crate) fn run(name: &str, args: &[String]) -> Result<String, String> {
    HANDLER.with_borrow_mut(|slot| match slot {
        Some(handler) => handler(name, args),
        None => Err("This app registers no commands, see hilen::app_commands".into()),
    })
}

#[cfg(all(test, feature = "inspect"))]
mod test {
    use super::*;

    #[test]
    fn a_registered_handler_answers_and_fails() {
        assert!(run("fly", &[]).is_err());
        register(|name, args| match name {
            "echo" => Ok(args.join(" ")),
            _ => Err(format!("no command {name}")),
        });
        assert_eq!(run("echo", &["a".into(), "b".into()]), Ok("a b".into()));
        assert_eq!(run("fly", &[]), Err("no command fly".into()));
    }
}
