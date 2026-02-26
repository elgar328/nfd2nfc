class Nfd2nfc < Formula
  desc "Convert filesystem entry names from NFD to NFC for cross-platform compatibility"
  homepage "https://github.com/elgar328/nfd2nfc"
  url "https://github.com/elgar328/nfd2nfc/releases/download/PLACEHOLDER_TAG/nfd2nfc-PLACEHOLDER_VERSION-universal-apple-darwin.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on :macos

  def install
    bin.install "nfd2nfc"
    prefix.install "nfd2nfc-watcher.app"
  end

  service do
    run [opt_prefix/"nfd2nfc-watcher.app/Contents/MacOS/nfd2nfc-watcher"]
    keep_alive crashed: true
    run_type :immediate
    working_dir HOMEBREW_PREFIX
  end

  test do
    assert_match "nfd2nfc", shell_output("#{bin}/nfd2nfc --version")
  end
end
