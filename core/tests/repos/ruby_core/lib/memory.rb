require_relative 'arena'

# vantage:
#   invariant: ThreadSafe
#   reason: Shared memory pool accessed by concurrent schedulers
class MemoryPool
  def initialize
    @arena = Arena.new
    @blocks = []
  end

  def allocate(size)
    start = @arena.alloc(0)
    size.times { @arena.alloc(0) }
    block_id = @blocks.length
    @blocks << [start, size]
    block_id
  end
end
