# frozen_string_literal: true

require "concurrent"

module Mavrik
  # Module that provides the task functionality when included in a class.
  #
  # @example
  #   class SayHello
  #     include Mavrik::Task
  #
  #     # Define instance method `call` to be executed by the task executor.
  #     def call(name, message:)
  #       puts "Hello, #{name}! #{message}"
  #     end
  #   end
  #
  #   # ...
  #
  #   # Calling the singleton method `call` will tell the task executor to run the task.
  #   SayHello.call("Alice", message: "How are you?")
  #
  module Task
    def self.included(base)
      base.extend(ClassMethods)
    end

    module ClassMethods
      def pipe
        p = TaskPipe.new(self)
        yield p if block_given?
        p.join
      end

      # Calls the task executor to run the task.
      # @param args [Array] The positional arguments to pass to the task
      # @param kwargs [Hash] The keyword arguments to pass to the task
      # @return [Future] The future object that will contain the result of the task
      def call(*args, **kwargs)
        task = JSON.generate(
          type: :new_task,
          queue: :default,
          definition: self.name,
          input_args: JSON.generate(args),
          input_kwargs: JSON.generate(kwargs)
        )

        task_id = Mavrik.client.send_message(task)

        Future.new(task_id:)
      end
    end

    class TaskPipe
      def initialize(task_class, futures = [])
        @task_class = task_class
        @futures = futures
      end

      # Specify the class of the task to call.
      # @param task_class [Class] The class of the task to call
      # @return [TaskPipe] The task pipe object
      def task(task_class)
        self.class.new(task_class, @futures)
      end

      # Calls the task executor to run the task.
      # @param args [Array] The positional arguments to pass to the task.
      # @param kwargs [Hash] The keyword arguments to pass to the task.
      def call(*args, **kwargs)
        @futures << Concurrent::Future.execute(executor: Mavrik.executor) do
          @task_class.call(*args, **kwargs)
        end
        nil
      end

      def join
        @futures.map(&:value)
      end
      private_methods :join
    end
  end
end
