# frozen_string_literal: true

# Run Rake task
require "rake"
Rake::Task.send(:load, "Rakefile")
# Rake::Task[:default].invoke

# Require gem
require "mavrik"
