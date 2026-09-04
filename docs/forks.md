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

- Guard `wantsExtendedDynamicRangeContent` behind iOS 16. Sent upstream as
  [gfx-rs/wgpu#10257](https://github.com/gfx-rs/wgpu/pull/10257) on 2026-09-03. Drop
  it from the fork once it ships in a release.
- Rename the published crates to the `hilen-` prefix. Fork only, never goes upstream.
- Tag the Metal layer with an explicit sRGB colorspace, 1 line in the match. Upstream sets
  nil, its comment says the layer default treats content as sRGB, but Apple's doc for
  `CAMetalLayer.colorspace` says the nil default means the content is not color matched,
  so sRGB content oversaturates on a P3 display. Upstream has the bug on file as
  [gfx-rs/wgpu#10013](https://github.com/gfx-rs/wgpu/issues/10013) since 2026-08-05,
  labeled bug by the maintainers, with a pixel measurement and no PR. The reporter asks
  for exactly this line, and dashscene applied the same call downstream in
  [driftsys/dashscene#750](https://github.com/driftsys/dashscene/pull/750). The PR is
  drafted but not sent, title `fix(metal): color match the sRGB surface color space`,
  Connections `Closes #10013`, the diff is the fork commit on trunk plus a changelog line.

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
release with 8 commits on top. `Pipeline::new` has 8 arguments there, 1 over the clippy
limit, since the gradient commit. Upstream candidates, none sent yet:

- Bump allocate the vertex buffer so `queue` and `draw` work several times per frame.
  Real bug in upstream, related to
  [Blatko1/wgpu-text#22](https://github.com/Blatko1/wgpu-text/issues/22).
- Expose custom layout queueing and glyph bounds, `queue_section_with_layout`,
  `process_queued`, `glyph_bounds_with_layout`. Small API addition, related to
  [Blatko1/wgpu-text#34](https://github.com/Blatko1/wgpu-text/issues/34).
- Gamma corrected blending on sRGB targets, a second fragment entry point picked by
  `TextureFormat::is_srgb`. Changes output for everyone on sRGB targets, needs a
  screenshot pair in the PR.

The second color per section, the stem darkening entry point and the `hilen-wgpu`
dependency stay in the fork.

## winit

Branch `tvos-0.30` at github.com/VladasZ/winit sits on the v0.30.13 tag, the newest
0.30 release, with 3 commits on top: tvos in the cfg aliases and target sections, the
5 view controller selectors guarded on tvos, the rename. Upstream master is 0.31 beta,
not a target. The same tvOS support is open upstream as
[rust-windowing/winit#4665](https://github.com/rust-windowing/winit/pull/4665) by
another contributor since 2026-08-10, it covers the same selectors plus CI and examples.
When it merges and a 0.30 release carries it, drop the 2 tvos commits from the fork.

## ffmpeg-next and ffmpeg-sys-next

Branch `hilen` at github.com/VladasZ/rust-ffmpeg and at
github.com/VladasZ/rust-ffmpeg-sys, each on the upstream v9.0.0 tag with 1 commit on
top, both git dependencies pinned by rev. The `video` feature links them, see
[video.md](video.md).

- `rust-ffmpeg-sys`: the build script downloads the static archives that
  `FFMPEG_DIR/prebuilt.txt` names for the target before it links them. Hilen's own
  convention, fork only. The same commit drops the `QTKit` framework from the macOS
  link line, the arm64 SDK no longer ships it and every link printed an
  `ld: ignoring file` warning. Upstream candidate, not sent yet.
- `rust-ffmpeg`: takes `ffmpeg-sys-next` from the sys fork by rev, so the tree holds
  one copy of the bindings. Fork only.

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
