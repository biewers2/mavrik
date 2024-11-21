# frozen_string_literal: true

module Mavrik
  # Allows configuration of the Mavrik server/client.
  module Configurable
    # Configure the Mavrik client.
    # @yield [c] The configuration object to update.
    # @yieldparam config [Config] The configuration object.
    # @yieldreturn [Config] The updated config object
    def configure
      @config = Config.new
      yield @config if block_given?
      @config
    end

    def config
      if defined?(@config)
        @config
      else
        raise Error, "#{self} not configured"
      end
    end

    def reset_config!
      remove_instance_variable(:@config) if defined?(@config)
    end
  end
end
