# Contributing

Contributions are welcome! This project is built with Rust and follows standard open-source practices.

## Development setup

```bash
git clone https://github.com/veduket/localtool.git
cd localtool
cargo build
cargo test
```

The project is a Cargo workspace. To build only a specific crate:

```bash
cargo build -p local-dns
cargo build -p local-ssl
cargo build -p localtool
```

## Code style

- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Write tests for new functionality
- Update documentation (this book) when adding or changing commands

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p local-dns
cargo test -p local-ssl
```

## Pull request workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a PR with a clear description of the changes

## Documentation

This book lives in `book/` and is built with mdbook:

```bash
cargo install mdbook
mdbook build book/   # generates book/book/
mdbook serve book/   # live preview at localhost:3000
```

## License

MIT — see [LICENSE](https://github.com/veduket/localtool/blob/main/LICENSE).
