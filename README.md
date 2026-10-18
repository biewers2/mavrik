# Mavrik

Decentralized, asynchronous task executor for Ruby programs.

## Architecture

There are three components to Mavrik:
- **Ruby gem**: serves as a thin wrapper to the extension
- **Rust extension**: defines the Mavrik client
- **Mavrik server**: the task manager and executor

The Mavrik server is a standalone binary that can be run independently of the Ruby gem. It is responsible for managing
and executing tasks submitted by clients. Although independent of the gem, the server is designed to run tasks in Ruby
through the [magnus](https://docs.rs/magnus/latest/magnus/) crate.

### Directory Structure

| Directory | Description |
|-----------|-------------|
| `/` | Project root directory; Ruby gem definition and Rust workspace |
| `/lib` | Ruby source code |
| `/spec` | Ruby test code|
| `/ext/mavrik-ext` | Rust extension crate; houses the client code |
| `/ext/mavrik-exe` | Rust binary crate; houses the entrypoint for the server |
| `/ext/mavrik-lib` | Rust library crate; houses the core logic used by the client/server |
| `/ext/ruby-bridge` | Utility crate containing Ruby FFI bindings |

## Development

The Ruby gem can be built and installed using `rake` commands. This will also build and install the Rust extension into
the Ruby source code.

For building the server, `cargo` commands can be used. Here is a common command for running the server with logging enabled:
```sh
RUST_LOG=debug cargo run
```

## Testing

The Ruby gem uses RSpec for testing. To run the tests, execute:
```sh
bundle exec rspec
```
