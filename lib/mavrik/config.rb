# frozen_string_literal: true

module Mavrik
  class Config
    attr_accessor :host,
                  :port

    def to_h
      {}.tap do |h|
        h[:host] = host if host
        h[:port] = port if port
      end
    end
  end
end
