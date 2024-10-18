# frozen_string_literal: true

module Mavrik
  class Future
    def initialize(task_id:)
      @task_id = task_id
    end

    def await
      raise NotImplementedError
    end
  end
end
