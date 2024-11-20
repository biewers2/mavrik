# frozen_string_literal: true

require "rspec_helper"

#
# This spec sends 1000 tasks to the Mavrik task executor.
#
# Each task has a large input payload. The goal is to measure the performance of the TCP communication by observing how
# long it takes to send all the tasks.
#

RSpec.describe "send 1000 large input tasks", server: true, performance: true do
  class CpuIntensiveTask
    include Mavrik::Task

    def call(args)
      args
    end
  end

  it "is fast" do
    task_count = 1000

    task_ids = CpuIntensiveTask.pipe do |p|
      task_count.times { p.call(random_args) }
    end

    expect(task_ids.size).to eq(task_count)
  end

  def random_args
    (0...10000).map { (65 + rand(26)).chr }.join
  end
end
