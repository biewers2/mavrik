# frozen_string_literal: true

require "rspec_helper"

RSpec.describe Mavrik::Task do
  class SayHello
    include Mavrik::Task

    def call(name, message:)
      "Hello, #{name}! #{message}"
    end
  end

  describe ".call" do
    it "sends a new task to the server" do
      client = instance_double(Mavrik::Client, new_task: "task_id")
      allow(Mavrik).to receive(:client).and_return(client)

      task_id = SayHello.call("John", message: "How are you?")

      expect(task_id).to eq("task_id")
      expect(client).to have_received(:new_task).with(
        definition: SayHello.name,
        args: ["John"],
        kwargs: {message: "How are you?"}
      )
    end

    it "can generate empty args and kwargs" do
      client = instance_double(Mavrik::Client, new_task: "task_id")
      allow(Mavrik).to receive(:client).and_return(client)

      task_id = SayHello.call

      expect(task_id).to eq("task_id")
      expect(client).to have_received(:new_task).with(
        definition: SayHello.name,
        args: [],
        kwargs: {}
      )
    end
  end
end
