# Homebrew Cask for Bapu Studio.
#
# This file lives here for reference/version-control, but Homebrew taps must live in a repo
# literally named "homebrew-<tapname>". To make `brew install` work for users, either:
#
#   (a) Create a new repo named "homebrew-bapu-studio" under this account, copy this file into
#       it at Casks/bapu-studio.rb, then users run:
#         brew tap joravar/bapu-studio
#         brew install --cask bapu-studio
#
#   (b) Skip the tap repo and let people install directly from this file's raw URL:
#         brew install --cask https://raw.githubusercontent.com/joravar/bapu-studio/main/packaging/homebrew/bapu-studio.rb
#
# Update `version` and `sha256` on every release. Compute a release asset's sha256 with:
#   shasum -a 256 Bapu-Studio-<version>-mac-arm64.dmg
# (run after downloading the real .dmg from the release page).
#
# Apple Silicon only for now -- the release workflow builds on a macos-latest (arm64) GitHub
# runner with no explicit Intel target, so there is currently no mac-x64 asset to point to.
# If an Intel build is added later, this can be split into arm/intel sha256 + arch variants.
cask "bapu-studio" do
  version "2.0.0"
  sha256 "REPLACE_WITH_SHA256_OF_Bapu-Studio-macos-arm64.dmg"

  url "https://github.com/joravar/bapu-studio/releases/download/v#{version}/Bapu-Studio-macos-arm64.dmg"
  name "Bapu Studio"
  desc "Pure-Rust native developer cockpit for APIs, 33 database engines, streams, and secrets"
  homepage "https://joravar.github.io/bapu-studio/"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "Bapu Studio.app"

  zap trash: [
    "~/Library/Application Support/Bapu Studio",
    "~/Library/Preferences/dev.bapustudio.app.plist",
    "~/Library/Saved Application State/dev.bapustudio.app.savedState",
  ]
end
