# Homebrew formula for the mvanderlinde/homebrew-yolt tap.
# Source of truth lives in the yolt repo under packaging/homebrew-tap/ — copy this tree into the tap repo root when publishing.

class Yolt < Formula
  desc "Undo destructive LLM actions: auto-backup before file changes, quick revert (macOS)"
  homepage "https://github.com/mvanderlinde/yolt"
  license any_of: ["MIT", "Apache-2.0"]

  # Stable install: after tagging yolt (e.g. v0.1.0), uncomment and fill url/sha256/version.
  #   curl -sL https://github.com/mvanderlinde/yolt/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
  # url "https://github.com/mvanderlinde/yolt/archive/refs/tags/v0.1.0.tar.gz"
  # sha256 "..."
  # version "0.1.0"

  head "https://github.com/mvanderlinde/yolt.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "yolt", shell_output("#{bin}/yolt --version")
  end
end
