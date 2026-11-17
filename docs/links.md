# Links that open the app

A link of the app opens the app, not the browser: an https link of its domain on
a phone, a link of its url scheme on a desktop, the address a page loaded on in a
browser. The code is in `hilen/src/system/links`, the server half in
`hilen-server/src/app_links.rs`.

## The app

```rust
use hilen::system::{Links, OpenedLink};

Links::on_open(self, move |link: OpenedLink| {
    // link.url is the whole link, link.path() is "invite/x7k2" for both
    // https://shopping.vladas.xyz/invite/x7k2 and shopping://invite/x7k2
});
```

- The action runs on the main thread, in a turn of the loop of its own, while the
  subscriber lives.
- A link that comes before anybody listens is kept and goes to the first
  subscriber. That is how the link of a cold start arrives.
- Each link is given once.
- A log line shows only the scheme, the host and the first part of the path. The
  rest of a link can be a secret, like the code of an invite.
- It is an event a view subscribes to, not a hook of `App`. A UI test runs under
  the app of the runner, a hook of the app under test would never be called.

`hilen.toml` names what the app takes:

```toml
[links]
domains = ["shopping.vladas.xyz"]
scheme = "shopping"
```

`scheme` is optional, it defaults to `project_name` in lower case with `-` for `_`.

## Each platform

- iOS: winit starts the app with no delegate, so the engine sets its own when the
  launch finishes. It takes universal links, `continueUserActivity`, and scheme
  links, `openURL`. The link of a cold start comes from the launch options, a
  second copy of it is dropped. `hilen-mobile` writes the `applinks:` entitlement
  and the url scheme into the project.
- Android: `hilen-mobile` writes a verified intent filter for the domains, 1 for
  the scheme, and `singleTask` on the activity. The activity hands a new intent
  on. The engine reads the intent when the app resumes or gets the focus, then
  puts a blank one in its place so the link is not given twice. The app id is
  still `com.example.<name>` from the template, that is the package name the
  server route needs.
- macOS: a packaged app gets the link as an Apple Event, the handler is set just
  before the launch finishes. A start from a terminal with the link as an
  argument works too.
- Windows and Linux: the installer and the desktop file register the scheme. A
  start with a link as its argument first offers it to the running app over a
  local port, the number is in `links.port` in the data folder. When that app
  answers, the new start quits and the running window comes to the front.
  Otherwise it opens with the link. Only an argument with the scheme of the app
  counts as a link. Headless runs and test runs skip all of it.
- Browser: the address the page loaded on, unless it is the root page with no
  query.

## The server

A phone opens a link of a domain in the app only when the domain proves it
belongs to the app. 1 line in the backend serves both files:

```rust
let links = AppLinks::new()
    .apple("ABCDE12345", "vladas.shopping")
    .android("com.example.shopping", ["14:6D:E9:...:5C"])?;
let app = base_routes("shopping").merge(app_links_mount(links));
```

It serves `/.well-known/apple-app-site-association` from the team id and the
bundle id, and `/.well-known/assetlinks.json` from the package name and the
SHA-256 of the signing key. Every path of the domain opens the app. A phone that
is not set up gets a 404. The App ID needs Associated Domains turned on in the
Apple developer portal.

## Tests

- `Opened link test` in `ui-test-suite`, on desktop, the iOS simulator and the
  browser. `hilen::ui_test::open_link(url)` fakes a link through the same path a
  link of the system takes.
- `cargo test` in the engine for the path rules, the log line, the link in the
  start arguments and the hand over between 2 starts.
- `hilen-server/tests/app_links.rs` for the 2 files.

Not proven yet: a real link on a real phone, a packaged Mac app started with
`open <scheme>://...`, and the Windows installer and Linux packages.
