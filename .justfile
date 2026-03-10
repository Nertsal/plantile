list:
    just --list

web command *ARGS:
    cargo geng {{command}} --platform web --release {{ARGS}}

# Build the Demo version of the game for all platforms
build-demo:
    just build-all-platforms ./target/release-demo --features demo

docker_image := "ctl-build-docker"

build-docker:
    docker build -t {{docker_image}} .

build-web *ARGS:
    # Itch-Web
    CARGO_TARGET_DIR=./target/web \
    cargo geng build --release --platform web {{ARGS}}
    cd ./target/web/geng && zip -FS -r ../../web.zip ./*

build-itch-bundle:
    just build-web
    rm -rf ./target/jampack
    mkdir -p ./target/jampack
    cp -r ./dev-assets/jampack.html ./target/jampack/index.html
    cp -r ./target/jam ./target/jampack/left
    cp -r ./target/web/geng ./target/jampack/right
    cd ./target/jampack && zip -FS -r ../itch-bundle.zip ./*

# publish-itch:
#     LEADERBOARD_URL=wss://{{server}} CARGO_TARGET_DIR=`pwd`/target/release-demo/web cargo geng build --release --platform web --out-dir `pwd`/target/release-demo/web --features itch --features demo
#     butler -- push `pwd`/target/release-demo/web nertsal/close-to-light:html5
