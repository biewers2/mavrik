# frozen_string_literal: true

require "json"

module Mavrik
  # Executes a task using the provided arguments.
  # Called natively by the Mavrik task executor.
  class ExecuteTask
    # @param ctx [Hash] The task context, containing the task definition, arguments, and keyword arguments.
    # @return [Hash] Resulting hash of the task execution.
    def call(ctx)
      task_def = resolve(ctx, :definition)
      task_args = JSON.parse(resolve(ctx, :args))
      task_kwargs = JSON.parse(resolve(ctx, :kwargs))
      task_kwargs.transform_keys!(&:to_sym)

      task_class = Object.const_get(task_def)
      result = task_class.new.call(*task_args, **task_kwargs)

      {
        type: :success,
        result: JSON.generate(result)
      }
    rescue => e
      {
        type: :failure,
        class: e.class.to_s,
        message: e.message,
        backtrace: e.backtrace
      }
    end

    private

    def resolve(ctx, key)
      value = ctx[key]
      raise Mavrik::Error, "Missing task #{key}" if value.nil?
      raise Mavrik::Error, "Task #{key} must be a #{type}" unless value.is_a?(String)
      value
    end
  end
end