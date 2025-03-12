# Contributing to the Intercooperative Network

Thank you for your interest in contributing to the Intercooperative Network (ICN)! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

All contributors are expected to adhere to our cooperative values:
- Mutual respect and solidarity
- Democratic participation
- Transparency and openness
- Non-discrimination and inclusivity

## How to Contribute

### Reporting Issues

If you encounter bugs or have feature requests:

1. Check if the issue already exists in the issue tracker.
2. If not, create a new issue with a clear description, steps to reproduce, and relevant information.

### Contributing Code

1. **Fork the repository**: Create your own fork of the ICN project.
2. **Create a branch**: Create a branch for your changes (`git checkout -b feature/your-feature` or `fix/your-fix`).
3. **Make your changes**: Implement your changes, ensuring they follow our coding standards.
4. **Test your changes**: Run tests to ensure your changes don't break existing functionality.
5. **Submit a pull request**: Push your changes to your fork and submit a pull request to the main repository.

### Pull Request Process

1. Ensure your code follows our coding standards and includes appropriate tests.
2. Update documentation to reflect your changes if necessary.
3. Your pull request will be reviewed by core contributors, who may suggest changes.
4. Once approved, your changes will be merged into the main codebase.

## Development Guidelines

### Coding Standards

- Follow Rust's official style guidelines.
- Use meaningful names for variables, functions, and types.
- Comment your code where necessary, especially for complex logic.
- Write tests for new functionality.

### Project Structure

The ICN project is organized as a Rust workspace with multiple crates:

- `crates/core`: Core functionality and common utilities
- `crates/identity`: Identity system (DIDs, credentials)
- `crates/governance`: Governance system (DSL, VM, voting)
- `crates/networking`: Network communication
- `crates/storage`: Distributed storage
- `crates/node`: Node implementation

### Testing

- Write unit tests for all new functionality.
- Ensure existing tests pass with your changes.
- Consider writing integration tests for feature interaction.

### Documentation

- Update relevant documentation when making changes.
- Document public APIs with Rust doc comments.
- Consider updating the dev-docs if significant architectural changes are made.

## Governance Process

The ICN project is governed by cooperative principles:

1. **Proposals**: Major changes should be proposed as RFCs in the discussions area.
2. **Discussion**: All stakeholders have the opportunity to discuss proposals.
3. **Decision-making**: Decisions are made through consensus when possible, or through voting when necessary.
4. **Implementation**: Approved proposals are implemented according to the roadmap.

## License

By contributing to ICN, you agree that your contributions will be licensed under the project's MIT or Apache 2.0 license.

## Contact

If you have questions about contributing, please reach out through:
- GitHub Discussions
- Matrix Chat (#icn:matrix.org)
- Email (icn-dev@example.org)

Thank you for helping build a cooperative digital future! 