# frozen_string_literal: true

require_relative "lib/mavrik/version"

Gem::Specification.new do |spec|
  spec.name = "mavrik"
  spec.version = Mavrik::VERSION
  spec.authors = ["Jacob Biewer"]
  spec.email = ["biewers2@gmail.com"]

  spec.summary = "Decentralized asynchronous task executor"
  spec.description = "Mavrik runs as its own server running a multi-threaded Ruby execution environment."
  spec.homepage = "https://github.com/biewers2/mavrik"
  spec.required_ruby_version = ">= 3.0.0"
  #spec.required_rubygems_version = ">= 3.3.11"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/biewers2/mavrik"

  spec.files = Dir.chdir(__dir__) do
    `git ls-files -z`.split("\x0").reject do |f|
      (File.expand_path(f) == __FILE__) ||
        f.start_with?(*%w[bin/ test/ spec/ features/ .git appveyor Gemfile])
    end
  end
  spec.bindir = "exe"
  spec.executables = spec.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/mavrik/Cargo.toml"]
end
