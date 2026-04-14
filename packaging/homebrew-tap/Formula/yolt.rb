# Homebrew formula for the mvanderlinde/homebrew-yolt tap.
# Source of truth lives in the yolt repo under packaging/homebrew-tap/ — copy this tree into the tap repo root when publishing.

class Yolt < Formula
  desc "Undo destructive LLM actions: auto-backup before file changes, quick revert (macOS)"
  homepage "https://github.com/mvanderlinde/yolt"
  license any_of: ["MIT", "Apache-2.0"]

  url "https://github.com/mvanderlinde/yolt/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "3146b3b80ba972cb256d7b967bfa4cf025fb9a331ded55b45d9debd89c7ee9a7"
  version "0.2.0"

  head "https://github.com/mvanderlinde/yolt.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "yolt", shell_output("#{bin}/yolt --version")
  end
end
