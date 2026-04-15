# Homebrew formula for the mvanderlinde/homebrew-yolt tap.
# Source of truth lives in the yolt repo under packaging/homebrew-tap/ — copy this tree into the tap repo root when publishing.

class Yolt < Formula
  desc "Undo destructive LLM actions: auto-backup before file changes, quick revert (macOS)"
  homepage "https://github.com/mvanderlinde/yolt"
  license any_of: ["MIT", "Apache-2.0"]

  url "https://github.com/mvanderlinde/yolt/archive/refs/tags/v0.3.0.tar.gz"
  sha256 "91722b6ae4b9be9b59617f0c8107fc9b98395d9917a43bccdcb1fb60424c0dc0"
  version "0.3.0"

  head "https://github.com/mvanderlinde/yolt.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "yolt", shell_output("#{bin}/yolt --version")
  end
end
