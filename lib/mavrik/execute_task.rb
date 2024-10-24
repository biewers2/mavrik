# frozen_string_literal: true

require "json"

module Mavrik
  # Executes a task using the provided arguments.
  # Called natively by the Mavrik task executor.
  class ExecuteTask
    # @param ctx [String] JSON string representing the task context.
    # @return [String] JSON string representing the result of the task execution.
    def call(ctx)
      ctx = JSON.parse(ctx)
      task_def = definition(ctx)
      task_args = args(ctx)
      task_kwargs = kwargs(ctx)
      task_kwargs.transform_keys!(&:to_sym)

      task_class = Object.const_get(task_def)
      result = task_class.new.call(*task_args, **task_kwargs)

      JSON.generate({
        type: :success,
        result:
      })
    rescue => e
      JSON.generate({
        type: :failure,
        class: e.class,
        message: e.message,
        backtrace: e.backtrace
      })
    end

    private

    def definition(ctx)
      definition = ctx["definition"]
      raise Mavrik::Error, "Missing task definition" if definition.nil?
      raise Mavrik::Error, "Task definition must be a string" unless definition.is_a?(String)
      definition
    end

    def args(ctx)
      args = ctx["args"]
      raise Mavrik::Error, "Missing task arguments" if args.nil?
      raise Mavrik::Error, "Task arguments must be an array" unless args.is_a?(Array)
      args
    end

    def kwargs(ctx)
      kwargs = ctx["kwargs"]
      raise Mavrik::Error, "Missing task keyword arguments" if kwargs.nil?
      raise Mavrik::Error, "Task keyword arguments must be a hash" unless kwargs.is_a?(Hash)
      kwargs
    end
  end
end