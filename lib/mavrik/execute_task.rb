# frozen_string_literal: true

require "json"

module Mavrik
  class ExecuteTask
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