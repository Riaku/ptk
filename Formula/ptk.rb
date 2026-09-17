# typed: false
# frozen_string_literal: true

# Homebrew formula for ptk - High-performance CLI proxy
# To install: brew tap Riaku/tap && brew install ptk
class Ptk < Formula
  desc "High-performance CLI proxy to minimize LLM token consumption"
  homepage "https://www.rtk-ai.app"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_intel do
      url "https://github.com/Riaku/ptk/releases/download/v#{version}/ptk-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_INTEL"
    end

    on_arm do
      url "https://github.com/Riaku/ptk/releases/download/v#{version}/ptk-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Riaku/ptk/releases/download/v#{version}/ptk-x86_64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_INTEL"
    end

    on_arm do
      url "https://github.com/Riaku/ptk/releases/download/v#{version}/ptk-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM"
    end
  end

  def install
    bin.install "ptk"
  end

  test do
    assert_match "ptk #{version}", shell_output("#{bin}/ptk --version")
  end
end
