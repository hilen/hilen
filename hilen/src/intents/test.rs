use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use pollster::block_on;

use super::{IntentOption, Intents, options_json, run};

#[test]
fn a_handler_gets_its_parameters_and_gives_the_answer() {
    Intents::on("add", |args| {
        let item = args.text("item").unwrap_or("nothing").to_string();
        let list = args.text("list").map(str::to_string);
        async move { Ok(format!("{item} in {list:?}")) }
    });

    let answer = block_on(run("add", r#"{"item":"milk","list":"7"}"#)).unwrap();
    assert_eq!(answer, r#"milk in Some("7")"#);

    // A parameter the user left out is not in the arguments.
    let answer = block_on(run("add", r#"{"item":"eggs"}"#)).unwrap();
    assert_eq!(answer, "eggs in None");
}

#[test]
fn the_work_of_the_closure_runs_before_the_answer_is_awaited() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    Intents::on("count", move |_| {
        counted.fetch_add(1, Ordering::Relaxed);
        async { Ok(String::new()) }
    });

    let answer = run("count", "{}");
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    block_on(answer).unwrap();
}

#[test]
fn an_intent_with_no_handler_answers_with_an_error() {
    let err = block_on(run("missing", "{}")).unwrap_err();
    assert_eq!(err.to_string(), "the app has no handler for missing");
}

#[test]
fn bad_parameters_answer_with_an_error() {
    Intents::on("bad", |_| async { Ok("ran".to_string()) });
    assert!(block_on(run("bad", r#"{"item":5}"#)).is_err());
    assert!(block_on(run("bad", "not json")).is_err());
}

#[test]
fn the_values_of_a_choice_are_json_for_the_swift() {
    Intents::options("list", || {
        vec![
            IntentOption::new(7, "Groceries"),
            IntentOption::new("b", "Home \"stuff\""),
        ]
    });
    assert_eq!(
        options_json("list"),
        r#"[{"id":"7","title":"Groceries"},{"id":"b","title":"Home \"stuff\""}]"#
    );
    assert_eq!(options_json("unknown"), "[]");
}
