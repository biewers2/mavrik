# frozen_string_literal: true

require_relative "mavrik/execute_task"
require_relative "mavrik/version"
require_relative "mavrik/mavrik"
require_relative "test"

module Mavrik
  attr_reader :client

  class TestC
    attr_accessor :host, :port

    def to_h
      {host:, port:}
    end
  end

  def self.configure
    config = TestC.new
    yield config if block_given?
    @client = self.init(config.to_h)
  end
end
