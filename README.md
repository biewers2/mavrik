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
| `/ext/mavrik-ext` | Rust extension crate; houses the client code |
| `/ext/mavrik-exe` | Rust binary crate; houses the entrypoint for the server |
| `/ext/mavrik-lib` | Rust library crate; houses the core logic used by the client/server |
| `/ext/ruby-bridge` | Utility crate containing Ruby FFI bindings |

## Development

The Mavrik server can be developed independently of the Ruby gem by running `cargo` directly.


TODO


## Installation

Install the gem and add to the application's Gemfile by executing:

    $ bundle add mavrik

If bundler is not being used to manage dependencies, install the gem by executing:

    $ gem install mavrik

## Usage

Run the Mavrik task executor by executing:

    $ bundle exec mavrik

## Development

```bash
bundle exec rake install
```

After checking out the repo, run `bin/setup` to install dependencies. You can also run `bin/console` for an interactive prompt that will allow you to experiment.

To install this gem onto your local machine, run `bundle exec rake install`. To release a new version, update the version number in `version.rb`, and then run `bundle exec rake release`, which will create a git tag for the version, push git commits and the created tag, and push the `.gem` file to [rubygems.org](https://rubygems.org).

### Development Notes

#### Environment Variables

* `RUST_LOG=<level>`
  * View Rust logs when the env-logger is used
  * Level can be `warn`, `info`, `debug`, `trace`
