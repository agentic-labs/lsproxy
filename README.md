<div align="center">
<a href="https://agenticlabs.com/"><img src="https://raw.githubusercontent.com/agentic-labs/.github/main/assets/logo.png" alt="Agentic Labs" title="Agentic Labs" align="center" height="150px" /></a>

# lsproxy - Precise code navigation via an API
<p align="center">
  <a href="https://discord.gg/EUFGjSawyk"a><img alt="discord" src="https://img.shields.io/discord/1296271531994775552" /></a>
  <img alt="license" src="https://img.shields.io/github/license/agentic-labs/lsproxy" />
  <a href="https://pypi.org/project/lsproxy-sdk/" a><img alt="pypi" src="https://img.shields.io/pypi/v/lsproxy-sdk" /></a>
</p>
</div>


   
## <a name="what-is-lsproxy">What is lsproxy?</a>

`lsproxy` offers IDE-like code analysis and navigation functionality in a docker container with a REST API.

It supports [multiple languages](#supported-languages) and resolves relationships between code symbols (functions, classes, variables) anywhere in the project - which can be used to help AI assistants navigate a codebase or build custom code RAG systems.

`lsproxy` runs [Language Servers](https://microsoft.github.io/language-server-protocol/) and [ast-grep](https://github.com/ast-grep/ast-grep) under the hood, giving you precise search results without the headache of configuring and integrating language-specific tooling.

For more info, please refer to our [API Reference](https://docs.lsproxy.dev/api-reference).

[![](https://mermaid.ink/img/pako:eNptUtFumzAU_RV0q0qdRKpAgAAPk6buZVInTau0h9ZV5YRrYhVsZJuuLMq_7xraNLQ1D9jnnHt8ru09bHWFUIJo9N_tjhsXXP9miqmAhqfuLhhc0X_DLTL4cu-5ibX9pja825FMOS4VmjsGje2Mfh4Y3E8iP55007ej0Z9xNtm8slRBdddc1T2vMbhB84TGzgx4TQpu3aI22M2ZThL17dePTzYMFouv3v1TnNezBBPWydM95xiq6tj40D5I9SBkg75jaZ2HNrqxgVSBh49pKhSWNEKq6kXjIamkk1odVeajiiA0qLY4ncTpjVCugMFP3SvHYAw59TUpKPCInYScEz7SHDEjMmHn54F1Q4Nvl-obasozzMRKiNA6ox-xPEt4scT4Xc1O01FMcpH6771nI1E5-yYRIoUQWjQtlxU9wr0vYOB26F9JSdOKm0cGTB1Ix3unbwa1hdKZHkPou4o7_C45PcMWSsEbS2jH1a3W7auIllDu4RnKJLtM0yJL82hdJKs4zUIYoIyj5WWeJlGyzKNslefr5BDCv9GAiCKOi6yIlnGeFkmxPvwHnPP5bQ?type=png)](https://mermaid.live/edit#pako:eNptUtFumzAU_RV0q0qdRKpAgAAPk6buZVInTau0h9ZV5YRrYhVsZJuuLMq_7xraNLQ1D9jnnHt8ru09bHWFUIJo9N_tjhsXXP9miqmAhqfuLhhc0X_DLTL4cu-5ibX9pja825FMOS4VmjsGje2Mfh4Y3E8iP55007ej0Z9xNtm8slRBdddc1T2vMbhB84TGzgx4TQpu3aI22M2ZThL17dePTzYMFouv3v1TnNezBBPWydM95xiq6tj40D5I9SBkg75jaZ2HNrqxgVSBh49pKhSWNEKq6kXjIamkk1odVeajiiA0qLY4ncTpjVCugMFP3SvHYAw59TUpKPCInYScEz7SHDEjMmHn54F1Q4Nvl-obasozzMRKiNA6ox-xPEt4scT4Xc1O01FMcpH6771nI1E5-yYRIoUQWjQtlxU9wr0vYOB26F9JSdOKm0cGTB1Ix3unbwa1hdKZHkPou4o7_C45PcMWSsEbS2jH1a3W7auIllDu4RnKJLtM0yJL82hdJKs4zUIYoIyj5WWeJlGyzKNslefr5BDCv9GAiCKOi6yIlnGeFkmxPvwHnPP5bQ)

## Key Features

- 🎯 **Precise Cross-File Code Navigation**: Find symbol definitions and references across your entire project.
- 🌐 **Unified API**: Access multiple language servers through a single API.
- 🛠️ **Auto-Configuration**: Automatically detect and configure language servers based on your project files.
- 📊 **Code Diagnostics**: (Coming Soon) Get language-specific lint output from an endpoint.
- 🌳 **Call & Type Hierarchies**: (Coming Soon) Query multi-hop code relationships computed by the language servers.
- 🔄 **Procedural Refactoring**: (Coming Soon) Perform symbol operations like `rename`, `extract`, `auto import` through the API.
- 🧩 **SDKs**: Libraries to get started calling `lsproxy` in popular languages.
    

## <a name="getting-started">Getting started</a>
The easiest way to get started is to run our tutorial! Check it out at [demo.lsproxy.dev](https://demo.lsproxy.dev)
It's also super easy to run `lsproxy` on your code! We keep the latest version up to date on Docker Hub, and we have a Python SDK available via `pip.`

### Install the sdk

```bash
pip install lsproxy-sdk
```
You can find the source for the SDK [here](https://github.com/agentic-labs/lsproxy-python-sdk)

### Run a container or add to compose
> :warning: Version 0.2.0 and newer: JWT authentication is enabled by default for endpoints. Set the JWT_SECRET variable to configure or set USE_AUTH=false to turn off.

```bash
docker run -p 4444:4444 -v $WORKSPACE_PATH:/mnt/workspace -e USE_AUTH=false agenticlabs/lsproxy
```

```dockerfile
services:
  lsproxy:
    image: agenticlabs/lsproxy
    ports:
      - "4444:4444"
    environment:
      - JWT_SECRET=shared_secret
    volumes:
      - ${WORKSPACE_PATH}:/mnt/workspace
```
### Configure an existing system
You can also configure an existing system to run `lsproxy`. Add the following line in your dockerfile or run it as part of a startup script
```bash
curl -sSL https://github.com/agentic-labs/lsproxy/releases/latest/download/install-lsproxy.sh | sh
```

### Explore your workspace!

```python
from lsproxy import Lsproxy

client = Lsproxy()
file_path = "relative/path/from/project/root.cpp"
symbols = client.definitions_in_file(file_path)
for symbol in symbols:
    print(f"{symbol.name} is defined in {file_path}")
```

## <a name="contributing">Building products with lsproxy</a>

If you're building AI coding agents or code RAG, or would like to use `lsproxy` in a commercial product, please reach out!

## <a name="contributing">Contributing</a>

We appreciate all contributions! You don't need to be an expert to help out.
Please see [CONTRIBUTING.md](https://github.com/agentic-labs/lsproxy/blob/main/CONTRIBUTING.md) for more details on how to get
started.

> Questions? Reach out to us [on Discord](https://discord.gg/WafeS3jN).

## <a name="community">Community</a>

We're building a community. Come hang out with us!

- 🌟 [Star us on GitHub](https://github.com/agentic-labs/lsproxy)
- 💬 [Chat with us on Discord](https://discord.gg/EUFGjSawyk)
- ✏️ [Start a GitHub Discussion](https://github.com/agentic-labs/lsproxy/discussions)
- 🐦 [Follow us on Twitter](https://twitter.com/agentic_labs)
- 🕴️ [Follow us on LinkedIn](https://www.linkedin.com/company/agentic-labs)
  
## <a name="supported-languages">Supported languages</a>

We're looking to add new language support or better language servers so let us know what you need!
|Language|Server|URL|
|:-|:-|:-|
|C/C++|`clangd`|https://clangd.llvm.org/|
|Golang|`gopls`|https://github.com/golang/tools/tree/master/gopls|
|Java|`jdtls`|https://github.com/eclipse-jdtls/eclipse.jdt.ls|
|Javascript|`typescript-language-server`|https://github.com/typescript-language-server/typescript-language-server|
|PHP|`phpactor`|https://github.com/phpactor/phpactor|
|Python|`jedi-language-server`|https://github.com/pappasam/jedi-language-server|
|Rust|`rust-analyzer`|https://github.com/rust-lang/rust-analyzer|
|Typescript|`typescript-language-server`|https://github.com/typescript-language-server/typescript-language-server|
|Your Favorite Language | Awesome Language Server | https://github.com/agentic-labs/lsproxy/issues/new |
