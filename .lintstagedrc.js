export default {
  "*.{js,jsx,ts,tsx}": [
    "eslint --fix --max-warnings=0",
    "prettier --write",
    () => "tsc --noEmit", // Run type checking on the whole project if any TS file changes
  ],
  "*.{json,css,md,html,yaml,yml}": ["prettier --write"],
  "src-tauri/**/*.rs": [
    // Run rustfmt on the whole project (fast enough usually)
    () => "cargo fmt --manifest-path src-tauri/Cargo.toml",
    // Run clippy on the whole project
    () =>
      "cargo clippy --manifest-path src-tauri/Cargo.toml -- -W clippy::pedantic -W clippy::nursery -D warnings",
    // Run tests (crucial as requested)
    () => "cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1",
  ],
};
