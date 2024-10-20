require "rspec_helper"

RSpec.describe Mavrik do
  describe ".main" do
    it "is defined in the ext" do
      expect(described_class.respond_to?(:main)).to eq(true)
    end
  end

  describe ".init" do
    it "is defined in the ext" do
      expect(described_class.respond_to?(:init)).to eq(true)
    end
  end

  describe ".client" do
    # it "raises an error when the client is not configured" do
    #   expect {
    #     described_class.client
    #   }.to raise_error Mavrik::Error, "Mavrik client not configured"
    # end
  end
end
