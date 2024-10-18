# frozen_string_literal: true

require "rspec_helper"

RSpec.describe "client sends task to server", feature: true do
  class TestTask
    def call(a, b, c:)
      a + b + c
    end
  end

  it "sends a new task to the server" do
    task = JSON.generate(
      type: :new_task,
      queue: :default,
      definition: TestTask.to_s,
      input_args: "[1, 2]",
      input_kwargs: "{\"c\": 3}"
    )

    id = with_executor { Mavrik.client.send_message(task) }

    expect(id).not_to be_nil
  end

  it "returns an error when the message is invalid" do
    task = JSON.generate(
      type: :old_task, # wrong
      queue: :default,
      definition: "Test",
      input_args: "[]",
      input_kwargs: "{}"
    )

    expect {
      with_executor { Mavrik.client.send_message(task) }
    }.to raise_error(Mavrik::Error, /unknown variant `old_task`/)
  end
end
