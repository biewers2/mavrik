# frozen_string_literal: true

# Run Rake task
require "rake"
Rake::Task.send(:load, "Rakefile")
# Rake::Task[:default].invoke

# Require gem
require "mavrik"

# Require helpers
require_relative "helpers/server_helpers"

RSpec.configure do |c|
  c.before(:suite) { start_server }
  c.after(:suite) { stop_server }
end
