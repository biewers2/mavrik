# frozen_string_literal: true

module Mavrik
  module Configurable
    # Configure the Mavrik client.
    # @yield [config] The configuration object.
    # @yieldparam config [Config] The configuration object.
    def configure
      @config = Config.new
      yield @config if block_given?
      @config
    end

    def reset
      remove_instance_variable(:@config) if defined?(@config)
    end

    def config
      if defined?(@config)
        @config
      else
        raise Error, "#{self} not configured"
      end
    end
  end
end
