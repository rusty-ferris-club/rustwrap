# Changelog

# master

* supporting `rustwrap --latest` for figuring out the next version by itself
* **BREAKING** homebrew now supports arm and intel, so template variables must carry an arch postfix:
  * `__URL__[arm64]` or `__URL__[x86]`
  * `__SHA__[arm64]` or `__SHA__[x86]`
* fixing archive logic
