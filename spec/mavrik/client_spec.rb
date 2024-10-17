require "rspec_helper"

RSpec.describe Mavrik::Client do
  before do
    Mavrik.configure do |c|
      c.host = "127.0.0.1"
      c.port = 3009
    end
  end

  describe "#submit_task" do
    it "is defined in the ext" do
      expect(described_class.respond_to?(:submit_task)).to eq(true)
    end
  end
end
