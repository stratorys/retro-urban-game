# Commands

## Build
`cargo build`

Example:
`cargo build`

## Run game with map
`cargo run -- play --map <path>`

Example:
`cargo run -- play --map assets/config/town.json`

## Open editor with existing map
`cargo run -- edit --map <path>`

Example:
`cargo run -- edit --map assets/config/town.json`

## Create new map in editor
`cargo run -- edit --map <path> --new --width <w> --height <h>`

Example:
`cargo run -- edit --map assets/config/new-town.json --new --width 20 --height 20`

## Replace existing map in editor
`cargo run -- edit --map <path> --new --width <w> --height <h> --overwrite`

Example:
`cargo run -- edit --map assets/config/town.json --new --width 20 --height 20 --overwrite`

## Run schema tests
`cargo test -p map-schema`

Example:
`cargo test -p map-schema`
