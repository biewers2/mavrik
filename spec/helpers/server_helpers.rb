# frozen_string_literal: true

require "socket"

def start_server
  $server_pid = wait_for_sigusr1 do
    Process.fork do
      Mavrik.main({
        host: "127.0.0.1",
        port: 3001,
        signal_parent_ready: true,
        thread_count: 1,
      })
    end
  end

  Mavrik.configure do |c|
    c.host = "127.0.0.1"
    c.port = 3001
  end
end

def stop_server(pid = $server_pid)
  Process.kill("INT", pid) rescue nil
  Process.wait(pid)
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

