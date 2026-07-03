# Contributing to channels-rs

Thank you for your interest in contributing to `channels-rs`! We welcome all contributions, including bug reports, feature requests, documentation improvements, and code optimizations.

The following guidelines are designed to help make the contribution process smooth and efficient for everyone.

## Code of Conduct

By participating in this project, you agree to abide by our standards of conduct:
- Be respectful, welcoming, and professional.
- Focus on constructive feedback and collaboration.
- Avoid personal attacks, harassment, or exclusionary behavior.

## How Can I Contribute?

### Reporting Bugs
If you find a bug or unexpected behavior, please check the [Issue Tracker](https://github.com/ryntric/channels-rs/issues) to see if it has already been reported. If not, open a new issue and include:
- A clear and descriptive title.
- Steps to reproduce the issue.
- Expected and actual behavior.
- Your environment details (Rust version, OS, crate version).
- A minimal reproducible example (if possible).

### Requesting Features
We are open to new ideas and feature proposals. To request a feature:
- Open an issue describing the problem you want to solve or the functionality you need.
- Explain *why* this feature would be useful to others.
- Discuss potential API designs or architectural approaches before starting work.

### Submitting Pull Requests
1. Fork the repository and clone it locally.
2. Create a new branch for your work (e.g., `feature/my-new-feature` or `fix/issue-123`).
3. Make your changes, ensuring they align with our style and testing standards.
4. Push the branch to your fork.
5. Open a Pull Request (PR) against the `master` branch of the main repository.
6. Provide a comprehensive description of the changes in the PR.

## Local Development Setup

To build and test `channels-rs`, you will need the Rust toolchain installed via [rustup](https://rustup.rs/).

### Building
To check if the project compiles successfully:
```bash
cargo check --all-targets --all-features
