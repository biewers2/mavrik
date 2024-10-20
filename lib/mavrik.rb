# frozen_string_literal: true

require_relative "mavrik/config"
require_relative "mavrik/execute_task"
require_relative "mavrik/future"
require_relative "mavrik/task"
require_relative "mavrik/version"
require_relative "mavrik/mavrik"

module Mavrik
  class Error < StandardError; end

  # Configure the Mavrik client.
  # @yield [config] The configuration object.
  # @yieldparam config [Config] The configuration object.
  def self.configure
    config = Config.new
    yield config if block_given?
    @@client = self.init(config.to_h)
  end

  def self.reset
    remove_class_variable(:@@client) if defined?(@@client)
  end

  def self.client
    if defined?(@@client)
      @@client
    else
      raise Error, "Mavrik client not configured"
    end
  end

  def self.executor
    @executor ||= begin
      min_threads = [2, Concurrent.processor_count].min
      max_threads = [2, Concurrent.processor_count].max * 4
      max_queue = [2, Concurrent.processor_count].max * 10
      fallback_policy = :caller_runs

      # Debug
      # puts "Executor configuration: #{min_threads}-#{max_threads} threads, #{max_queue} queue size, #{fallback_policy} fallback policy"

      Concurrent::ThreadPoolExecutor.new(min_threads:, max_threads:, max_queue:, fallback_policy:)
    end
  end
end
