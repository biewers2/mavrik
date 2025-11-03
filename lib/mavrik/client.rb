# frozen_string_literal: true

require "singleton"

module Mavrik
  class Client
    include ::Singleton

    # Create a new Mavrik client connected to the server.
    def initialize
      @conn = Mavrik::Connection.new(Mavrik.config.to_h)
    end

    # Sends the "new task" request to the server
    # @param definition [String] The name of the task to run
    # @param args [Array] The positional arguments to pass to the task
    # @param kwargs [Hash] The keyword arguments to pass to the task
    # @return [String] The task ID
    def new_task(definition:, args:, kwargs:)
      @conn.request({
        type: :new_task,
        queue: :default,
        payload: {
          definition:,
          args: JSON.generate(args),
          kwargs: JSON.generate(kwargs)
        }
      })
    end

    def store_state
      @conn.request(type: :get_store_state)
    end
  end
end
