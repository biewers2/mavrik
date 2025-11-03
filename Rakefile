# frozen_string_literal: true

require "bundler/gem_tasks"
require "rb_sys/extensiontask"
require "rspec/core/rake_task"

GEMSPEC = Gem::Specification.load("mavrik.gemspec")

RbSys::ExtensionTask.new("mavrik-ext", GEMSPEC) do |ext|
  ext.lib_dir = "lib/mavrik"
end

RSpec::Core::RakeTask.new(:spec) do |t|
  t.pattern = Dir.glob("spec/**/*_spec.rb") # Specifies the pattern for test files
  t.rspec_opts = "--format documentation" # Optional RSpec command-line options
end

task build: :compile
task test: :spec
task default: %w[build test]
