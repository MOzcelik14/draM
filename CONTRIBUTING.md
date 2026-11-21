# Contributing to draM.

Thank you for your interest in contributing to **draM.**, part of the **prisM. Suite**.

## Code of Conduct

Keep all discussions, issues, and pull requests constructive, concise, and respectful.

## Development Setup

1. Install the stable Rust toolchain (`rustc` and `cargo`, Rust 2021 edition).
2. On Debian or Ubuntu Linux, install the required graphics and windowing development libraries:

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  pkg-config \
  libx11-dev \
  libgl1-mesa-dev \
  libxkbcommon-dev \
  libwayland-dev
```

3. Clone the repository and verify the workspace builds and tests pass:

```bash
git clone https://github.com/MOzcelik14/draM.git
cd draM
cargo test --workspace --locked
```

## Coding Standards

- **Memory Safety:** `#![forbid(unsafe_code)]` is enforced across the entire workspace. Do not introduce `unsafe` blocks or unsafe dependencies.
- **Formatting and Linting:** All code must pass `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- **Localization:** Never hardcode user-facing strings in `dram-app`. Add matching keys to both `locales/en/main.ftl` and `locales/tr/main.ftl`. The test suite enforces key parity (`tr_and_en_bundles_have_matching_message_keys`) and checks for hardcoded labels (`ui_sources_contain_no_hardcoded_user_labels`).
- **Turkish Text Rules:** Use the language-aware casing and collation helpers in `dram-core::text` (`to_upper_for_lang`, `to_lower_for_lang`, `cmp_tr`) rather than Rust's locale-unaware `to_uppercase()` / `to_lowercase()` when processing screenplay content.
- **Documentation:** Preserve existing doc comments and keep documentation clear, factual, and free of emojis or third-party product references.

## Pull Request Process

1. Fork the repository and create a branch from `main`.
2. Use conventional commit prefixes (`feat:`, `fix:`, `docs:`, `chore:`, `ci:`).
3. Add unit or integration tests covering your changes.
4. Run the full verification suite before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
bash -n scripts/package-linux.sh
```

5. Open a pull request against `main` with a clear summary of the changes and how they were tested.
