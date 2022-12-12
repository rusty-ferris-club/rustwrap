<h1 align="center">
   <img src="media/rustwrap.svg" width="160"/>
   <br/>
   Rustwrap
</h1>
<p align="center">
<img src="https://github.com/rusty-ferris-club/rustwrap/actions/workflows/build.yml/badge.svg"/>
</p>



A tool that helps wrap binary releases for easy distribution. Currently supporting:

* **npm** - `npm install -g your-tool` will make your binary `your-tool` available via the CLI. `rustwrap` creates the necessary binary packages and root package with a Node.js shim that delegates running to your platform-specific bin.
* **Homebrew** - creates a recipe and saves or publishes it to your tap.





## Download

For macOS:

```
brew tap rusty-ferris-club/tap && brew install rustwrap
```
Through cargo:

```
cargo install rustwrap
```

Otherwise, grab a release from [releases](https://github.com/rusty-ferris-club/rustwrap/releases) and run `rustwrap --help`:


## Getting started

Build a single `rustwrap.yaml`, and describe which releases you have an where to get them per platform, and your provider blocks.

Use `__VERSION__` when you want the actual version replaced.

```yaml
targets:
  - platform: win32
    arch: x64
    url_template: https://github.com/rusty-ferris-club/recon/releases/download/v__VERSION__/recon-x86_64-windows.zip
  - platform: linux
    arch: x64
    url_template: https://github.com/rusty-ferris-club/recon/releases/download/v__VERSION__/recon-x86_64-linux.tar.xz
  - platform: darwin
    arch: x64
    url_template: https://github.com/rusty-ferris-club/recon/releases/download/v__VERSION__/recon-x86_64-macos.tar.xz
  - platform: darwin
    arch: x64
    url_template: https://github.com/rusty-ferris-club/recon/releases/download/v__VERSION__/recon-aarch64-macos.tar.xz

# provider: npm
# both recon-root.json and recon-sub.json paths are relative to working folder
npm:
  publish: false # dont publish to npm, just generate the packages on disk
  org: "@recontools"
  name: recon
  root: 
    name: recon-tool
    manifest: rustwrap/fixtures/config/recon-root.json
    readme: rustwrap/fixtures/config/README.md
  sub: 
    manifest: rustwrap/fixtures/config/recon-sub.json
    readme: rustwrap/fixtures/config/README.md

# provider: homebrew
brew:
  name: recon
  publish: true # push an update commit to the tap repo
  tap: jondot/homebrew-tap
  recipe_fname: recon.rb
  recipe_template: |
    class Recon < Formula
      desc "recon"
      homepage "http://www.example.com"
      url "__URL__"
      version "__VERSION__"
      sha256 "__SHA__"

      def install
        bin.install "recon"
      end
    end
```
With your `rustwrap.yaml` and relevant files in the current working folder, run:

```
$ rustwrap --tag 0.6.0
```

The `--tag` value replaces the `__VERSION__` value.

# About

This tool was inspired in part by the [Rome toolchain and infrastructure](https://github.com/rome/tools) built for releasing Rome on `npm`. 

I gave it some generic abilities (downloading releases independently) and tweaks, and extended it with a Homebrew provider, something which I needed for a while now.

* It can be used for any self-contained binary produced in any language, not just Rust
* Accepting PRs for more providers

# Contributing

We are accepting PRs. Feel free to [submit PRs](https://github.com/rusty-ferris-club/rustwrap/pulls).

To all [Contributors](https://github.com/rusty-ferris-club/rustwrap/graphs/contributors) - you make this happen, thanks!

# License

Copyright (c) 2022 [@jondot](http://twitter.com/jondot). See [LICENSE](LICENSE.txt) for further details.
