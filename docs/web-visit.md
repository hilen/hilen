# Visit record of a page

A page that freezes the browser or makes it die runs no more script, so it
cannot send anything at that moment. Sentry and the panic hook see nothing.
The visit record in `hilen/src/web_visit.rs` covers that case.

## Turning it on

An app returns a path on the origin of its page from `App::visit_report_path`,
like `Some("/api/visit")`. The default is `None`, and then nothing is kept or
sent. Native builds never read it. The app's backend takes a POST of JSON on
that path.

## What it keeps

1 record per visit, in local storage under the key `hilen_visit`:

- `id`, `started` in Unix milliseconds, `url`, `agent`, `screen` as
  `<width>x<height> at <pixel ratio>`,
- `steps`, each with a `name` and `ms` since the start: `start`, `instance`
  before the adapter request, `adapter <backend> <name> <driver> <driver info>`,
  `device`, `surface`, `ui`, `first frame`, and `panic`,
- `alive_ms` and `frames`, the time the page ran and the frames it presented,
- `hidden`, the tab was in the background at the last write,
- `closed`, `pagehide` came, the visit ended normally,
- `log`, the last 40 log lines, each cut to 300 characters.

A step is written at once, before the next call can hang. A heartbeat writes
the record every 5 seconds. A log line is only kept in memory and goes out with
the next step or heartbeat, a log line can come often.

## What it sends

The body is `{"kind": ..., "panic": ..., "visit": <the record>}`.

- `unfinished`: the next start of the page found a record with `closed` false.
  The last step and `alive_ms` say how far the visit got and how long it ran.
  With `hidden` true it may be a phone that killed a background tab, not a
  crash.
- `panic`: the panic hook sends it at once, with the panic text in `panic`, and
  marks the visit closed so the next start does not send it again.

Both go out as a sync request, the panic hook has to deliver before the
instance dies.

## Checking it

A page with the inspector, or any release page, shows `Visit record <id>
started` in the console at start. A visit that did not end shows `The last
visit did not end, it stopped after <step>` at the next start, and
`Sent the unfinished visit report`. Killing the tab from the task manager of
the browser leaves such a record.
