# frozen_string_literal: true

require "bundler/gem_tasks"
require "rb_sys/extensiontask"

task build: :compile

GEMSPEC = Gem::Specification.load("mavrik.gemspec")

RbSys::ExtensionTask.new("mavrik", GEMSPEC) do |ext|
  ext.lib_dir = "lib/mavrik"
end

task default: :compile
