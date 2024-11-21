# frozen_string_literal: true

require_relative "mavrik/client"
require_relative "mavrik/config"
require_relative "mavrik/configurable"
require_relative "mavrik/execute_task"
require_relative "mavrik/executor"
require_relative "mavrik/task"
require_relative "mavrik/version"
require_relative "mavrik/mavrik"

module Mavrik
  extend Executor
  extend Configurable

  class Error < StandardError; end

  # The Mavrik client instance.
  # @return [Mavrik::Client] The client instance.
  def self.client
    Client.instance
  end
end
