# vantage:
#   invariant: AppendOnly
#   reason: Prevent rollback corruption in memory arena
class Arena
  def initialize
    @items = []
    @generation = 0
  end

  def alloc(item)
    @items << item
    @items.length - 1
  end

  def get(index)
    @items[index]
  end
end
