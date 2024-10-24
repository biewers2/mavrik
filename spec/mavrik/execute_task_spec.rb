# frozen_string_literal: true

require "rspec_helper"

RSpec.describe Mavrik::ExecuteTask do
  class TestTask
    def call
      6
    end
  end

  describe "#call" do
    it "calls a new instance of the defined class" do
      allow(TestTask).to receive(:new).and_call_original

      ctx = JSON.generate({
        definition: "TestTask",
        args: [],
        kwargs: {}
      })

      result = subject.call(ctx)

      expect(JSON.parse(result)).to eq("type" => "success", "result" => 6)
      expect(TestTask).to have_received(:new).with(no_args)
    end

    it "passes the parsed arguments to the new defined instance" do
      ctx = JSON.generate({
        definition: "TestTask",
        args: [1, 2],
        kwargs: {c: 3}
      })
      test_task_class = double("TestTask.class")
      test_task = double("TestTask", call: 6)
      allow(Object).to receive(:const_get).and_return(test_task_class)
      allow(test_task_class).to receive(:new).and_return(test_task)

      result = subject.call(ctx)

      expect(JSON.parse(result)).to eq("type" => "success", "result" => 6)
      expect(test_task).to have_received(:call).with(1, 2, c: 3)
    end

    it "returns an error message on error" do
      ctx = JSON.generate({
        definition: "TestTask",
        args: [1, 2],
        kwargs: {c: 3}
      })
      allow(TestTask).to receive(:new).and_raise(StandardError.new("error message"))

      result = subject.call(ctx)

      expect(JSON.parse(result)).to include(
        "type" => "failure",
        "class" => StandardError.name,
        "message" => "error message"
      )
    end
  end
end
