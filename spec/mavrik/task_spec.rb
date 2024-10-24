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
      client = instance_double(Mavrik::Client, send_message: "\"task_id\"")
      allow(Mavrik).to receive(:client).and_return(client)
      allow(Mavrik::Future).to receive(:new).and_call_original

      task = SayHello.call("John", message: "How are you?")

      expect(task).to be_a_kind_of(Mavrik::Future)
      expect(client).to have_received(:send_message).with(JSON.generate(
        type: :new_task,
        queue: :default,
        ctx: JSON.generate({
          definition: SayHello.name,
          args: ["John"],
          kwargs: {message: "How are you?"}
        })
      ))
      expect(Mavrik::Future).to have_received(:new).with(task_id: "task_id")
    end

    it "can generate empty args and kwargs" do
      client = instance_double(Mavrik::Client, send_message: "\"task_id\"")
      allow(Mavrik).to receive(:client).and_return(client)

      task = SayHello.call

      expect(task).to be_a_kind_of(Mavrik::Future)
      expect(client).to have_received(:send_message).with(JSON.generate(
        type: :new_task,
        queue: :default,
        ctx: JSON.generate({
          definition: SayHello.name,
          args: [],
          kwargs: {}
        })
      ))
    end
  end

  describe "#await" do
    it "returns the completed task" do
      # client = instance_double(Mavrik::Client, send_message: "task_id")
      # allow(Mavrik).to receive(:client).and_return(client)
      # allow(Mavrik::Future).to receive(:new).and_call_original
      #
      # task = SayHello.call("John", message: "How are you?")
      #
      # expect(task).to be_a_kind_of(Mavrik::Future)
      # expect(client).to have_received(:send_message).with(JSON.generate(
      #   type: :new_task,
      #   queue: :default,
      #   ctx: JSON.generate({
      #     definition: SayHello.name,
      #     args: ["John"],
      #     kwargs: {message: "How are you?"}
      #   })
      # ))
      # expect(Mavrik::Future).to have_received(:new).with(task_id: "task_id")
    end
  end
end
