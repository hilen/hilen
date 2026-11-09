//! Actions the system runs in the app without its screen, like a sentence
//! said to Siri. iOS only for now, see docs/intents.md.
//!
//! ```ignore
//! use hilen::intents::{IntentOption, Intents};
//!
//! Intents::on("add_item", |args| {
//!     let item = args.text("item").unwrap_or_default().to_string();
//!     save(&item);
//!     async move { Ok(format!("Added {item}")) }
//! });
//! Intents::options("list", || lists().map(|l| IntentOption::new(l.id, l.name)).collect());
//! ```
//!
//! The intents and their phrases are the `[[intent]]` tables of `hilen.toml`,
//! `hilen-mobile` makes their Swift. On iOS the system can start the app in
//! the background for an intent, with no window and no `setup` of a view, so
//! an app registers its handlers in `App::before_launch`.

#[cfg(target_os = "ios")]
pub(crate) mod ios;

use std::{cell::RefCell, collections::HashMap, future::Future, pin::Pin};

use anyhow::Result;
#[cfg_attr(not(target_os = "ios"), cfg(test))]
use anyhow::anyhow;
#[cfg_attr(not(target_os = "ios"), cfg(test))]
use log::{info, warn};
use serde::{Deserialize, Serialize};

/// What a handler gives back: the answer the system shows or speaks, or an
/// error it says instead.
type Answer = Pin<Box<dyn Future<Output = Result<String>> + Send>>;
type Handler = Box<dyn Fn(IntentArgs) -> Answer>;
type Options = Box<dyn Fn() -> Vec<IntentOption>>;

#[derive(Default)]
struct Registry {
    handlers: HashMap<String, Handler>,
    options:  HashMap<String, Options>,
}

thread_local! {
    // The system calls in on the main thread, and the handlers touch the
    // state of the app, which lives there.
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// The values of the parameters of 1 run, each as text. A choice gives the
/// id of its `IntentOption`. A parameter the user left out is not there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct IntentArgs {
    values: HashMap<String, String>,
}

impl IntentArgs {
    pub fn text(&self, parameter: &str) -> Option<&str> {
        self.values.get(parameter).map(String::as_str)
    }

    #[cfg_attr(not(target_os = "ios"), cfg(test))]
    fn parse(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// 1 value of a choice parameter, like 1 list of a shopping app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntentOption {
    /// What the handler gets back in `IntentArgs`.
    pub id:    String,
    /// What the user says and sees.
    pub title: String,
}

impl IntentOption {
    pub fn new(id: impl ToString, title: impl Into<String>) -> Self {
        Self {
            id:    id.to_string(),
            title: title.into(),
        }
    }
}

pub struct Intents;

impl Intents {
    /// Whether the system runs intents of this app. Only iOS for now.
    pub const fn is_supported() -> bool {
        cfg!(target_os = "ios")
    }

    /// Runs `handler` for the intent `name` of `hilen.toml`, on the main
    /// thread. The work that needs the state of the app goes in the closure,
    /// the future may wait for the network. Its text is the answer of the
    /// system. A second call for the same name takes the place of the first.
    pub fn on<F, A>(name: &str, handler: F)
    where
        F: Fn(IntentArgs) -> A + 'static,
        A: Future<Output = Result<String>> + Send + 'static, {
        REGISTRY.with_borrow_mut(|registry| {
            registry
                .handlers
                .insert(name.to_string(), Box::new(move |args| Box::pin(handler(args))));
        });
    }

    /// The values of the choice parameter `parameter`, asked on the main
    /// thread each time the system needs them.
    pub fn options(parameter: &str, provider: impl Fn() -> Vec<IntentOption> + 'static) {
        REGISTRY.with_borrow_mut(|registry| {
            registry.options.insert(parameter.to_string(), Box::new(provider));
        });
    }

    /// Tells the system that the values of a choice changed, like a new
    /// list. Siri then knows the new names in the phrases.
    pub fn options_changed() {
        #[cfg(target_os = "ios")]
        ios::options_changed();
    }
}

#[cfg_attr(not(target_os = "ios"), cfg(test))]
/// Starts the intent `name`, on the main thread. The answer comes from the
/// future, which needs no main thread.
fn run(name: &str, args_json: &str) -> Answer {
    let args = match IntentArgs::parse(args_json) {
        Ok(args) => args,
        Err(err) => {
            warn!("intent {name} came with bad parameters {args_json:?}: {err:#}");
            return Box::pin(async move { Err(err) });
        }
    };
    info!("intent {name} runs with {:?}", args.values);
    let answer = REGISTRY.with_borrow(|registry| registry.handlers.get(name).map(|handler| handler(args)));
    answer.unwrap_or_else(|| {
        warn!("intent {name} has no handler, the app calls Intents::on in before_launch");
        let name = name.to_string();
        Box::pin(async move { Err(anyhow!("the app has no handler for {name}")) })
    })
}

#[cfg_attr(not(target_os = "ios"), cfg(test))]
/// The values of a choice as JSON, `[]` when the app gave none.
fn options_json(parameter: &str) -> String {
    let options = REGISTRY.with_borrow(|registry| registry.options.get(parameter).map(|provider| provider()));
    let options = options.unwrap_or_else(|| {
        warn!("choice {parameter} has no values, the app calls Intents::options in before_launch");
        Vec::new()
    });
    serde_json::to_string(&options).unwrap_or_else(|err| {
        warn!("the values of choice {parameter} do not make JSON: {err}");
        "[]".to_string()
    })
}

#[cfg(test)]
mod test;
