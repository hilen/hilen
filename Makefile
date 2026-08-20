
include build/common.mk

render:
	cargo run -p render-test

ui:
	rust ./build/ui-test.rs

uui:
	cargo run -p ui-test --release -- --headless

# One test per engine pillar, the fast pre-commit check. Desktop only,
# debug, headless. Full lanes still run for rendering-heavy changes.
SMOKE_TESTS = TestColorChecker,ButtonPress,InjectTouch,AnchorLayoutTest,TableView2Test,WheelScrollTest,LabelFitText,LabelFont,ImageView,NineSegment,GameView,AnimationDrivesFrames,ShowModally,BackdropBlurTest

smoke:
	cargo run -p ui-test -- --headless --test-name $(SMOKE_TESTS)

ui-ios:
	rust ./build/ios/sim-test.rs

ui-ios-human:
	rust ./build/ios/sim-test.rs --human

all:
	order
	make wasm
	make ios
	make ui
	make render

fix:
	cargo fix --allow-dirty --allow-staged --all

.PHONY: bench
bench:
	cargo run -p bench --release

mobile:
	cargo install test-mobile
	test-mobile --path=../test-mobile/mobile-template

OS := $(shell uname)

# objc2 reads IPHONEOS_DEPLOYMENT_TARGET at compile time. Anything from 13.0 up
# turns `available!(ios = 13.0)` into a constant true, which drops wgpu's runtime
# guard and sends `supportsFamily:` to GPUs that have no such selector.
IOS_TARGET := IPHONEOS_DEPLOYMENT_TARGET=12.0

build-ios:
ifeq ($(OS), Darwin)
	env CFLAGS="" SDKROOT="" $(IOS_TARGET) cargo lipo -p test-game
else
	@echo " build-ios can only be run on macOS."
endif

ios-debug:
	env $(IOS_TARGET) cargo lipo -p test-game
	rm -f ./target/universal/release/libtest_game.a
	cp ./target/universal/debug/libtest_game.a ./target/universal/release/libtest_game.a

fix-lint:
	cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets

.PHONY: ci
ci:
	typos
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p test-game --features bench --all-targets -- -D warnings
	cargo machete

lint:
	cargo clippy --workspace --all-targets -- -D warnings

serve:
	rustup target add wasm32-unknown-unknown
	command -v trunk >/dev/null || cargo install --locked trunk
	cd ./test-game && trunk serve --features webgl --address 0.0.0.0 --port 44800

serve-release:
	rustup target add wasm32-unknown-unknown
	command -v trunk >/dev/null || cargo install --locked trunk
	cd ./test-game && trunk serve --features webgl --release --address 0.0.0.0 --port 44800

serve-size:
	rustup target add wasm32-unknown-unknown
	command -v trunk >/dev/null || cargo install --locked trunk
	cd ./test-game && trunk serve --features webgl --cargo-profile=size --address 0.0.0.0 --port 44800

wasm:
	rustup target add wasm32-unknown-unknown
	command -v trunk >/dev/null || cargo install --locked trunk
	cd ./test-game && trunk build

# The suite in a real installed browser, driven over the inspect socket.
BROWSER ?= chrome
ui-web:
	rustup target add wasm32-unknown-unknown
	rustup component add rust-src
	command -v trunk >/dev/null || cargo install --locked trunk
	bun build/web/drive.ts --browser $(BROWSER)

.PHONY: import
import:
	cargo fix --allow-dirty --allow-staged --all --all-targets

.PHONY: mobile
