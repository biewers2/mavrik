class TestTask
  include Mavrik::Task

  def call(name, message:)
    "Hello, #{name}! #{message}"
  end
end
