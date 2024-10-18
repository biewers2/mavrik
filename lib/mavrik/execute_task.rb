# frozen_string_literal: true

require "json"

module Mavrik
  # Executes a task using the provided arguments.
  # Called natively by the Mavrik task executor.
  class ExecuteTask
    # @param definition [String] Class string of the task to execute.
    # @param input_args [String] JSON array string of the positional arguments.
    # @param input_kwargs [String] JSON object string of the keyword arguments.
    # @return [Hash] The result of the task execution.
    def call(definition:, input_args:, input_kwargs:)
      args = JSON.parse(input_args)
      kwargs = JSON.parse(input_kwargs)
      kwargs.transform_keys!(&:to_sym)

      task_class = Object.const_get(definition)
      result = task_class.new.call(*args, **kwargs)

      {
        type: "success",
        result:
      }
    rescue => e
      {
        type: "error",
        class: e.class,
        message: e.message,
        backtrace: e.backtrace
      }
    end
  end
end