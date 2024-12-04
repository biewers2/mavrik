# Mavrik

Decentralized, asynchronous task executor.

## Installation

Install the gem and add to the application's Gemfile by executing:

    $ bundle add mavrik

If bundler is not being used to manage dependencies, install the gem by executing:

    $ gem install mavrik

## Usage

Run the Mavrik task executor by executing:

    $ bundle exec mavrik

## Development

After checking out the repo, run `bin/setup` to install dependencies. You can also run `bin/console` for an interactive prompt that will allow you to experiment.

To install this gem onto your local machine, run `bundle exec rake install`. To release a new version, update the version number in `version.rb`, and then run `bundle exec rake release`, which will create a git tag for the version, push git commits and the created tag, and push the `.gem` file to [rubygems.org](https://rubygems.org).

### Development Notes

#### Environment Variables

* `RUST_LOG=<level>`
  * View Rust logs when the env-logger is used
  * Level can be `warn`, `info`, `debug`, `trace`
* `LD_LIBRARY_PATH=<path-to-shared-objects>`
  * On Ubuntu 24 with Ruby installed using `asdf`, the compiler was having a hard time finding `libruby.so` on its own
  * Specifying the `lib/` directory of where the shared object file is was necessary for cargo-related things to work

## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/biewers2/mavrik.
