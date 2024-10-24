# frozen_string_literal: true

require "rspec_helper"

class SuccessTask
  include Mavrik::Task

  def call(a, b, c:)
    a + b + c
  end
end

class FailureTask
  include Mavrik::Task

  def call
    raise StandardError, "Task failed"
  end
end

RSpec.describe "task runs and can be awaited", server: true, feature: true do
  it "can call a successful task" do
    task_future = SuccessTask.call(1, 2, c: 3)
    expect(task_future.task_id).not_to be_nil

    result = task_future.await
    expect(result).to eq(6)
  end

  it "can call a failing task" do
    task_future = FailureTask.call
    expect(task_future.task_id).not_to be_nil

    expect {
      task_future.await
    }.to raise_error(StandardError, "Task failed")
  end
end
