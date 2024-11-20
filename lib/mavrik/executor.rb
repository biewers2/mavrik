# frozen_string_literal: true

module Mavrik
  module Executor
    def executor
      @executor ||= begin
        min_threads = [2, Concurrent.processor_count].min
        max_threads = [2, Concurrent.processor_count].max * 4
        max_queue = [2, Concurrent.processor_count].max * 10
        fallback_policy = :caller_runs

        Concurrent::ThreadPoolExecutor.new(min_threads:, max_threads:, max_queue:, fallback_policy:)
      end
    end
  end
end
