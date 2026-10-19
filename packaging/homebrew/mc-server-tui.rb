class McServerTui < Formula
  desc "Minecraft Java server manager with a terminal UI and resident supervisor"
  homepage "https://github.com/JUANWANG-BUAA/mc-server-tui"
  version "{{VERSION}}"

  on_arm do
    url "{{URL_ARM64}}"
    sha256 "{{SHA256_ARM64}}"
  end

  on_intel do
    url "{{URL_X86_64}}"
    sha256 "{{SHA256_X86_64}}"
  end

  def install
    bin.install "bin/mc", "bin/mc-server-tui", "bin/mc-server-tui-daemon", "bin/mc-server-tui-cli"
  end

  test do
    assert_match "mc-server-tui", shell_output("#{bin}/mc version --json")
  end
end
