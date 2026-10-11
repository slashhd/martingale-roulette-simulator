# Third-party notices

The application source is licensed under MIT. Dependencies retain their own licenses.

## Rust dependencies

`Cargo.toml` declares Axum, Tokio, Serde, serde_json, Rand, Rayon, Chrono and CSV. `Cargo.lock` records the resolved packages and transitive dependencies.

Release packaging collects package names, versions, license identifiers and available license/notice files from Cargo's downloaded crate directories into `THIRD_PARTY_LICENSES.txt` in each archive. Development-only packages may also appear in that list. The Rust standard library/toolchain has its own notices available from [the Rust project](https://github.com/rust-lang/rust).

For source builds, dependencies and their license files are downloaded through Cargo.

## Browser assets

The HTML references external assets that are fetched directly by the browser:

| Asset | Source | License |
| --- | --- | --- |
| Tailwind CSS browser CDN | [Tailwind CSS](https://github.com/tailwindlabs/tailwindcss) | MIT |
| Chart.js 4.4.8 | [Chart.js](https://github.com/chartjs/Chart.js/releases/tag/v4.4.8) | MIT |
| Plus Jakarta Sans | [Font project](https://github.com/tokotype/PlusJakartaSans) | SIL Open Font License 1.1 |
| JetBrains Mono | [Font project](https://github.com/JetBrains/JetBrainsMono) | SIL Open Font License 1.1 |

Browser assets are not copied into the executable or release archive. Full styling, fonts and charts require access to their CDN hosts.
