# frozen_string_literal: true

module Mavrik
  class Future
    attr_reader :task_id

    def initialize(task_id:)
      @task_id = task_id
    end

    def await
      result_str = Mavrik.client.send_message(JSON.generate(type: :await_task, task_id:))
      result = JSON.parse(result_str)

      if result["type"] == "success"
        result["result"]
      else
        error_class = Object.const_get(result["class"])
        error_message = result["message"]
        raise error_class, error_message
      end
    end
  end
end
