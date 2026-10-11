# Changelog

## 0.1.0

### Added

- English user interface, errors, comments and documentation.
- Deterministic simulation tests, HTTP API/storage tests and dependency-free UI tests.
- Configuration for local port, CSV path and Rayon worker count.
- CI on Windows, Linux and macOS.
- Release packaging with dependency notices and checksums.
- Tag-triggered draft GitHub releases.

### Changed

- HTML and JavaScript are embedded in standalone binaries.
- All profit-target and until-bust batches use cancellable background jobs.
- Profit-target mode has a consistent 100,000-spin safety cap.
- Statistics and timeline sample sizes are shown explicitly.
- Documentation distinguishes loss, bust, target and capped outcomes.
- Launchers use their own directory; source launches rebuild with the lockfile.
- Opening the page waits for the user to start a simulation.

### Fixed

- Input validation for unknown modes/rules, spin limits, targets and non-finite values.
- Error/cancellation states being overwritten by a success message.
- Hardcoded 100% bust history entries when sessions reached a safety cap.
- Non-finite unlimited limits in history JSON.
- Rounded and manually quoted CSV writes.
- CSV exports with mixed legacy layouts.
- Unbounded retention of completed job results.
