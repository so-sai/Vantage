#!/usr/bin/env ruby
# vantage:
#   invariant: Idempotent
#   reason: Deploy script must be safe to retry

class Deployer
  def initialize(env)
    @env = env
  end

  def run
    puts "Deploying to #{@env}"
    build_backend
    build_frontend
    deploy_all
  end

  private

  def build_backend
    puts "Building Rust backend..."
  end

  def build_frontend
    puts "Building TSX frontend..."
  end

  def deploy_all
    puts "Deploying #{@env}..."
  end
end

Deployer.new(ARGV[0] || "staging").run
