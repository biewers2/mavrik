# frozen_string_literal: true

module Mavrik
  class Client
    include Singleton

    # Create a new Mavrik client connected to the server.
    # @param conn [Connection] The connection to the server.
    def initialize
      @conn = Mavrik.init(Mavrik.config.to_h)
    end

    # Sends the "new task" request to the server
    # @param definition [String] The name of the task to run
    # @param args [Array] The positional arguments to pass to the task
    # @param kwargs [Hash] The keyword arguments to pass to the task
    # @return [String] The task ID
    def new_task(definition:, args:, kwargs:)
      task = JSON.generate(
        type: :new_task,
        queue: :default,
        ctx: JSON.generate({
          definition:,
          args:,
          kwargs:
        })
      )

      task_id_str = @conn.send_message(task)
      JSON.parse(task_id_str)
    end

    def store_state
      state_str = @conn.send_message(JSON.generate(type: :get_store_state))
      JSON.parse(state_str)
    end
  end

  def self.client
    Client.instance
  end
end
