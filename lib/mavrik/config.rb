# frozen_string_literal: true

module Mavrik
  # Configuration object for the Mavrik server/client.
  class Config
    # @!attribute host [String] The TCP hsot of the Mavrik server.
    attr_accessor :host

    # @!attribute port [Integer] The TCP port of the Mavrik server.
    attr_accessor :port

    # @!attribute rb_thread_count [Integer] The number of Ruby threads to spin up.
    attr_accessor :rb_thread_count

    # @!attribute signal_parent_ready [Boolean] Whether to signal the parent process when the server is ready to accept connections.
    attr_accessor :signal_parent_ready

    def to_h
      {}.tap do |h|
        h[:host] = host if host
        h[:port] = port if port
        h[:rb_thread_count] = rb_thread_count if rb_thread_count
        h[:signal_parent_ready] = signal_parent_ready if signal_parent_ready
      end
    end
  end
end
