require_relative 'memory'

# vantage:
#   invariant: DeterministicOrdering
#   reason: Task execution order must be reproducible
class Scheduler
  def initialize
    @pool = MemoryPool.new
    @tasks = []
  end

  def spawn(name)
    @pool.allocate(name.length)
    @tasks << name
  end

  def run_all
    @tasks.each { |task| puts "Running: #{task}" }
  end
end
