# Forks

Hilen replaces 5 crates.io crates with forks, all declared in the root `Cargo.toml`.
`wgpu` and `winit` come from crates.io under the `hilen-` prefix, `wgpu_text`,
`ffmpeg-next` and `ffmpeg-sys-next` are git dependencies pinned by rev. Every fork clone lives in `~/dev/forks/<name>` with an
`upstream` remote next to `origin`. Why each fork exists is in [ios.md](ios.md) for wgpu
and wgpu_text and in [tvos.md](tvos.md) for winit. This file tracks what each fork
carries against upstream and how a fix goes upstream.

Before a rebase, tag the old head of every fork, `pin-<date>`, and push the tag. Hilen
pins `wgpu_text` by rev, and a rev that no branch reaches any more fails to fetch. The
last rebase was on 2026-09-03, the old heads are `pin-2026-09-03`.

## wgpu

Branch `ios-edr-guard` at github.com/VladasZ/wgpu sits on the upstream v30.0.1 tag with
3 commits on top. Only 1 source file differs from stock wgpu 30, Apple only, so Windows,
Linux, Android and wasm builds of `hilen-wgpu` are identical to upstream.

- Guard `wantsExtendedDynamicRangeContent` behind iOS 16. Merged upstream as
  [gfx-rs/wgpu#10257](https://github.com/gfx-rs/wgpu/pull/10257) on 2026-09-16, on
  trunk only, so v31 is the first release with it.
- Rename the published crates to the `hilen-` prefix. Fork only, never goes upstream.
- Tag the Metal layer with an explicit sRGB colorspace, 1 line in the match. Upstream sets
  nil, its comment says the layer default treats content as sRGB, but Apple's doc for
  `CAMetalLayer.colorspace` says the nil default means the content is not color matched,
  so sRGB content oversaturates on a P3 display. Upstream had the bug on file as
  [gfx-rs/wgpu#10013](https://github.com/gfx-rs/wgpu/issues/10013). The same line was
  merged upstream as [gfx-rs/wgpu#10286](https://github.com/gfx-rs/wgpu/pull/10286) by
  another contributor on 2026-09-29, on trunk only. No PR from us.

Both fixes are on trunk, which is v31, and neither is copied to the upstream `v30`
branch. The decision is to wait for v31, not to send a backport. The upstream v31
milestone is due 2026-10-09. Once hilen moves to a v31 that holds both, the fork carries
only the rename, so hilen goes back to stock `wgpu` and the `hilen-wgpu` crates stop.
`wgpu_text` moves in the same step, its fork depends on `hilen-wgpu` and two `wgpu`
crates do not interchange, see [ios.md](ios.md). Check on an iOS 12 device and on a P3
display first.

The 2026-09-03 rebase dropped the null WebGPU adapter commit, upstream v30.0.1 fixed it
through wasm-bindgen 0.2.127. Upstream trunk is v31 work with breaking changes, the
rebase target stays the newest v30 tag until hilen moves major.

Publishing follows upstream's habit, only the crates whose content changed get the new
version and the internal version requirements stay at `30.0.0`. The 2026-09-03 rebase
touched `wgpu` and `wgpu-hal`, so crates.io has `hilen-wgpu` and `hilen-wgpu-hal` at
30.0.2 while `hilen-wgpu-core` and the 4 `hilen-wgpu-core-deps-*` crates stay at 30.0.0.
The workspace version bump is its own commit on the branch. Publish order is
`hilen-wgpu-hal` first, then `hilen-wgpu`, each `cargo publish -p <name>` from a clean
tree. Hilen then takes them with `cargo update -p hilen-wgpu -p hilen-wgpu-hal`, and
`cargo update -p cfg_aliases` since upstream v30.0.1 needs 0.2.2. Check with
`cargo tree -d --depth 0 -p demo` that no wgpu crate is duplicated.

## wgpu-text

Branch `master` at github.com/VladasZ/wgpu-text sits on upstream master at the v30.0.0
release with 12 commits on top. `Pipeline::new` has 8 arguments there, 1 over the clippy
limit, since the gradient commit. The examples do not build there, they still import
`wgpu_text` and the crate is renamed. 3 commits are the upstream candidates, in the
order they go upstream:

- Expose custom layout queueing and glyph bounds, `queue_custom_layout`,
  `process_queued`, `glyph_bounds_custom_layout`, named after the `glyph_brush`
  methods they wrap. Merged upstream as
  [Blatko1/wgpu-text#47](https://github.com/Blatko1/wgpu-text/pull/47) on 2026-09-23,
  released in v30.0.1.
- Several `queue` and `draw` calls per frame. The vertex buffer is bump allocated, a
  plain `queue` or `process_queued` starts it over and `queue_append` or
  `process_queued_append` writes after the earlier batch, so a forgotten call shows the
  wrong text at once and never leaks. `Font::begin_frame` and its `first_batch` flag
  pick the call in hilen. It answers
  [Blatko1/wgpu-text#22](https://github.com/Blatko1/wgpu-text/issues/22) and
  [Blatko1/wgpu-text#34](https://github.com/Blatko1/wgpu-text/issues/34).
- Earlier batches keep their glyphs when the cache texture is full. A later batch
  whose texture writes land on a glyph an earlier batch of the frame still draws
  writes into a copy of the texture, and the next frame starts with a texture of
  double size. Without it 2 labels at 72 pixels break on the default 256 by 256
  texture, the reason the maintainer gave up on several draws in
  [Blatko1/wgpu-text#17](https://github.com/Blatko1/wgpu-text/issues/17). See
  [text.md](text.md).

2 more commits came from the LG TV, see [webos.md](webos.md):

- The glyph cache forks from memory. The cache keeps its pixels, one byte a texel, and
  writes the forked texture from them. The texture to texture copy it replaced made the
  TV drop its WebGL context.
- Brushes share one set of text pipelines. `shared.rs` builds the shader module, both
  pipelines and the bind group layout once per device, target format, depth state,
  sample count and stem darkening, kept per thread. A brush per font used to build its
  own, about 1.2 seconds each on the TV.

The last 2 of the upstream candidates went upstream together, 1 commit on the branch `multi-draw` on top of
upstream v30.0.1, sent as
[Blatko1/wgpu-text#48](https://github.com/Blatko1/wgpu-text/pull/48) on 2026-10-02.
That commit has shorter comments than the 2 on `master`, the code does the same.

The gamma corrected blending commit was dropped on 2026-09-19. It only acted on sRGB
targets and hilen renders into plain Unorm since 2026-07-26, see
[colors.md](colors.md), so it never ran. The old heads of that day are
`pin-2026-09-19`, `pin-2026-09-19b` and `pin-2026-09-19c`.

The newest commit is `BrushBuilder::with_multithread`. It passes the `multithread`
switch of `glyph_brush` through, so a hot build keeps the glyph cache off the global
rayon pool, see [hot-reload.md](hot-reload.md).

The second color per section, the stem darkening entry point, the effect pipeline
for outlines and soft shadows, see [text.md](text.md), and the `hilen-wgpu`
dependency stay in the fork.

## winit

Branch `tvos-0.30` at github.com/VladasZ/winit sits on the v0.30.13 tag, the newest
0.30 release, with 4 commits on top: tvos in the cfg aliases and target sections, the
5 view controller selectors guarded on tvos, the rename, and the feature `ios-attach`.
crates.io has it as `hilen-winit` 0.30.14.

`ios-attach` lets the event loop run on a `UIApplication` that someone else started,
and leave it again: `EventLoopExtIOSAttach::run_app_attached`, `detach` and
`release_classes` in `winit::platform::ios`. A hot build needs it, see
[hot-reload.md](hot-reload.md). Fork only so far, upstream 0.30 takes no features. The
fork cannot be checked by itself, its dev dependency on itself has the old name, so the
proof is a build of hilen with a path dependency, then `far crates-publish -p
hilen-winit`. Upstream master is 0.31 beta,
not a target. The same tvOS support is open upstream as
[rust-windowing/winit#4665](https://github.com/rust-windowing/winit/pull/4665) by
another contributor since 2026-08-10, it covers the same selectors plus CI and examples.
When it merges and a 0.30 release carries it, drop the 2 tvos commits from the fork.

## ffmpeg-next and ffmpeg-sys-next

Branch `hilen` at github.com/VladasZ/rust-ffmpeg and at
github.com/VladasZ/rust-ffmpeg-sys, each on the upstream v9.0.0 tag with a few commits
on top, both git dependencies pinned by rev. The `video` feature links them, see
[video.md](video.md).

- `rust-ffmpeg-sys`: with no `FFMPEG_DIR` and a static link, the build script downloads
  the archive that its own `prebuilt.txt` names for the target into `OUT_DIR`, headers
  and libraries, and links that, plus what the archive's `lib/link.txt` lists, like
  dav1d and zlib. Hilen's own convention, fork only. 5 commits on top. The first commit drops the `QTKit` framework from the macOS
  link line, the arm64 SDK no longer ships it and every link printed an
  `ld: ignoring file` warning. Upstream candidate, not sent yet.
- `rust-ffmpeg`: takes `ffmpeg-sys-next` from the sys fork by rev, so the tree holds
  one copy of the bindings. Fork only. 5 commits on top.

## Updating a fork to the newest upstream

```bash
cd ~/dev/forks/<name>
git tag pin-<date> <branch> && git push origin pin-<date>
git fetch upstream --tags
git rebase --onto <upstream tag> <old base> <branch>
```

The rename commit conflicts in `Cargo.toml` on `repository` and `version`, keep the
fork's repository and the new upstream version. Its `Cargo.lock` conflict is resolved by
taking the upstream lock, `git checkout --ours Cargo.lock`, then `cargo update -w
--offline`, which drops the trimmed workspace members without moving any version. After
the rename the packages are `hilen-wgpu-hal` and friends, so every cargo command uses
those names.

Proof before pushing is hilen itself. Add a temporary patch section to hilen's root
`Cargo.toml`, never committed:

```toml
[patch.crates-io]
hilen-wgpu = { path = "../forks/wgpu/wgpu" }
hilen-wgpu-core = { path = "../forks/wgpu/wgpu-core" }
hilen-wgpu-hal = { path = "../forks/wgpu/wgpu-hal" }
hilen-wgpu-core-deps-apple = { path = "../forks/wgpu/wgpu-core/platform-deps/apple" }
hilen-wgpu-core-deps-emscripten = { path = "../forks/wgpu/wgpu-core/platform-deps/emscripten" }
hilen-wgpu-core-deps-wasm = { path = "../forks/wgpu/wgpu-core/platform-deps/wasm" }
hilen-wgpu-core-deps-windows-linux-android = { path = "../forks/wgpu/wgpu-core/platform-deps/windows-linux-android" }

[patch."https://github.com/VladasZ/wgpu-text"]
hilen-wgpu-text = { path = "../forks/wgpu-text" }
```

`cargo tree -p demo --duplicates` must show 1 copy of every wgpu crate, then `make
smoke`. Path crates show their own warnings, crates.io crates hide them, so an upstream
`expect(unused)` in wgpu-core shows up here and is not ours. Restore `Cargo.toml` and
`Cargo.lock` with `git checkout` afterwards, then force push the fork branch.

## Sending a fork fix upstream

The fork is a real GitHub fork, so a PR can come from any branch of it. The PR branch
must start from upstream trunk, never from the fork branch, or the rename commit rides
along. This is how #10257 was made.

```bash
cd ~/dev/forks/wgpu
git fetch upstream trunk
git switch -c <branch> upstream/trunk
git apply <fix>.patch
```

The patch carries only the fix plus one `CHANGELOG.md` line under Unreleased, Bug
Fixes, the backend heading, with `#NNNN` as the PR number. Gates before the commit, all
on the toolchain wgpu pins in its `rust-toolchain.toml`:

```bash
cargo fmt --check -p wgpu-hal
cargo clippy -p wgpu-hal --features metal -- -D warnings
rustup target add aarch64-apple-ios --toolchain <pinned>
cargo check -p wgpu-hal --features metal --target aarch64-apple-ios
```

Then commit, push to the fork and open the PR against trunk:

```bash
git push -u origin <branch>
gh pr create --repo gfx-rs/wgpu --base trunk --head VladasZ:<branch> --title '...' --body-file body.md
```

Put the printed number into the changelog line, amend, `git push --force-with-lease`.

The PR template has Connections, Description, Testing, Squash or Rebase and a checklist.
Only Description and Testing are needed. Recent external PRs landed without the rest and
the template itself says ticking the boxes is not required. Connections stays only when
a real issue exists, as `Closes #NNNN`. Squash or Rebase only matters with more than 1
commit. Description says what is
missing, where it is called, what happens and why skipping it is safe, in a few plain
sentences. Testing names the device and OS version it ran on. The title follows the
recent metal fixes, `fix(metal): ...`. The 2024 PR
[#5744](https://github.com/gfx-rs/wgpu/pull/5744) is the same shape with a pasted crash
log, paste one when there is one.

After the PR the clone sits on the PR branch. Switch back to `ios-edr-guard` before any
fork work for hilen.
