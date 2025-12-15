# GTA:SA Path Data Extractor

The extractor is based on the reverse engineering of the GTA community. The following link provides more information about the paths in GTA:SA: https://gtamods.com/wiki/Paths_(GTA_SA)

## Usage

1. Create the `paths` directory.
2. Download GTA:SA game files.
3. Copy all NODES*.DAT from `data/paths` to the `paths` directory.
4. Run `cargo run --example import -- --uri bolt://localhost:7687 --username neo4j --password PASSWORD`
