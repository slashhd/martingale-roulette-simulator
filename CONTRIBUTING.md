# Contributing

Bug reports, documentation improvements and pull requests are welcome.

For a bug report, include your operating system, Rust version (for source builds), simulation parameters, expected behavior and actual behavior. Attach a small anonymized example if a CSV import fails.

Before committing, enable the repository's [pre-commit hook](docs/DEVELOPMENT.md#commit-hook-and-test-report). It runs the full check suite and updates `docs/TEST_REPORT.md`; when the suite passes, the hook stages the report with the commit. You can run the same suite at any time with `python scripts/run_checks.py`.

Before a pull request:

1. Read the [development guide](docs/DEVELOPMENT.md) and [simulation model](docs/SIMULATION_MODEL.md).
2. Keep user-facing text, comments and documentation in English.
3. Add deterministic tests for changed simulation rules or API/storage behavior, following the [automatic test discovery conventions](docs/DEVELOPMENT.md#checks).
4. Confirm the committed [test report](docs/TEST_REPORT.md) shows a passing run, and use the current CI-generated report and checks for the pull request revision during review.
5. Update documentation and `CHANGELOG.md` when behavior changes.

Keep pull requests focused. Explain the problem, resulting behavior and checks performed. Do not commit generated history, release archives, credentials or build artifacts.

Contributions are distributed under the project's [MIT license](LICENSE).
