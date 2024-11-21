require "rspec_helper"

RSpec.describe "Mavrik is configurable" do
  after(:each) do
    Mavrik.reset_config!
  end

  it "allows configuration of the Mavrik server/client" do
    Mavrik.configure do |c|
      c.host = "1.2.3.4"
      c.port = 1212
      c.signal_parent_ready = true
      c.rb_thread_count = 8
    end

    expect(Mavrik.config.host).to eq("1.2.3.4")
    expect(Mavrik.config.port).to eq(1212)
    expect(Mavrik.config.signal_parent_ready).to eq(true)
    expect(Mavrik.config.rb_thread_count).to eq(8)
  end

  it "raises an error if the Mavrik server/client is not configured" do
    expect { Mavrik.config }.to raise_error(Mavrik::Error)
  end

  it "resets the configuration of the Mavrik server/client" do
    Mavrik.configure do |c|
      c.host = "localhost"
      c.port = 1212
    end

    Mavrik.reset_config!

    expect { Mavrik.config }.to raise_error(Mavrik::Error)
  end

  it "returns a hash representation of the configuration" do
    Mavrik.configure do |c|
      c.host = "localhost"
      c.port = 1212
    end

    expect(Mavrik.config.to_h).to eq({host: "localhost", port: 1212})
  end

  it "returns an empty hash if no configuration is specified" do
    Mavrik.configure

    expect(Mavrik.config.to_h).to eq({})
  end
end