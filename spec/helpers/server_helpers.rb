# frozen_string_literal: true

require "socket"

# Run code with the Mavrik task executor running.
def with_executor
  port = 3001

  pid = wait_for_sigusr1 do
    Process.fork do
      Mavrik.main({
        host: "127.0.0.1",
        port:,
        signal_parent_ready: true,
        thread_count: 2,
      })
    end
  end

  Mavrik.configure do |c|
    c.host = "127.0.0.1"
    c.port = port
  end

  result = yield if block_given?

  Process.kill("INT", pid)
  Process.wait(pid)

  result
ensure
  # Process.kill("INT", pid) rescue nil
  Mavrik.reset
end

# Wait for the server to tell us its ready to receive TCP connections.
def wait_for_sigusr1
  t = Thread.new do
    curr_thr = Thread.current
    trap("USR1") { curr_thr.kill }
    sleep
  end

  pid = yield if block_given?

  t.join
  pid
end

