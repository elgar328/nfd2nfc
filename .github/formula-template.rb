class Nfd2nfc < Formula
  desc "Convert filesystem entry names from NFD to NFC for cross-platform compatibility"
  homepage "https://github.com/elgar328/nfd2nfc"
  url "https://github.com/elgar328/nfd2nfc/archive/refs/tags/PLACEHOLDER_TAG.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args(path: "nfd2nfc")
    system "cargo", "install", *std_cargo_args(path: "nfd2nfc-watcher")

    system "/usr/bin/codesign", "-f", "-s", "-",
           "--identifier", "io.github.elgar328.nfd2nfc", bin/"nfd2nfc"
    system "/usr/bin/codesign", "-f", "-s", "-",
           "--identifier", "io.github.elgar328.nfd2nfc-watcher", bin/"nfd2nfc-watcher"
  end

  test do
    assert_match "nfd2nfc", shell_output("#{bin}/nfd2nfc --version")
    assert_match "nfd2nfc-watcher", shell_output("#{bin}/nfd2nfc-watcher --version")
  end
end
