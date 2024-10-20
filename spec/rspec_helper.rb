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
  c.before(feature: true) { start_server }
  c.after(feature: true) { stop_server }

  c.before(performance: true) { start_server }
  c.after(performance: true) { stop_server }
end
