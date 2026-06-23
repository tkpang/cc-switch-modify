# Windows 构建（GNU 工具链）

本 fork（lepro-connect）在 **Windows 上用 GNU 工具链**（`x86_64-pc-windows-gnu`）构建，**不用 MSVC**。

**为什么不用 MSVC**：构建机带宽受限，Visual Studio C++ Build Tools（~1.5GB+ SDK）在该机器上装不动/不收敛。GNU 工具链更轻，验证可完整构建 cc-switch（Tauri 2.8 + webview2-com + windows-rs + sqlite + quickjs 全栈在 gnu 下编译+链接通过）。

## 一次性环境准备（Windows 构建机）

1. **Rust（rustup）** + GNU toolchain。`rust-toolchain.toml` 已 pin `channel = "stable-x86_64-pc-windows-gnu"`，clone 后自动用 gnu。若 rustup 默认 host 是 msvc，可 `rustup set default-host x86_64-pc-windows-gnu` 或直接安装 `rustup toolchain install stable-x86_64-pc-windows-gnu`。

2. **w64devkit（完整 mingw-w64：gcc 16 + 全套 binutils）** — rustup 自带的 `self-contained` mingw **缺 `as`**，且新 GCC 无独立 `libgcc_eh.a`，故需完整 mingw：
   - 下载 `w64devkit-x64-<ver>.7z.exe`（GitHub: skeeto/w64devkit/releases）。**GitHub 在国内/本构建机上很慢** → 改在公司服务器 `192.168.33.13`（外网快）下载，再经内网 `scp -3` 中转到 Windows（见下「慢网中转」）。
   - 自解压：`w64devkit-x64-<ver>.7z.exe -y`（在 `C:\` 下运行 → 得 `C:\w64devkit`）。
   - 把 `C:\w64devkit\bin` 加到 **PATH 最前**（提供 `gcc`/`as`/`dlltool`/`ld`）。

3. **`libgcc_eh.a` 桩** — GCC 16 把异常处理并进了 `libgcc.a`，无独立 `libgcc_eh.a`，但 rust 的 gnu target 仍注入 `-lgcc_eh` → 链接报 `cannot find -lgcc_eh`。建一个**空桩**即可（真正的 eh 符号由同时链接的 `libgcc.a` 提供）：
   ```
   ar crs "C:\w64devkit\lib\gcc\x86_64-w64-mingw32\<gccver>\libgcc_eh.a"
   ```

4. **Node + pnpm**：`npm i -g pnpm`；国内用快镜像：`pnpm config set registry https://registry.npmmirror.com`；然后 `pnpm install`。

5. **build.rs**：`src-tauri/build.rs` 里嵌入 Common Controls manifest 用的 `/MANIFEST:*` 是 **MSVC link.exe 专用** flag，GNU `ld` 不认。本 fork 已把它 gate 到 `target_env = "msvc"`，gnu 下跳过（不影响 WebView2 应用）。

## 构建

```
pnpm install                     # 首次
pnpm build:renderer              # 前端 vite → dist
cd src-tauri && cargo build      # 验证 Rust/Tauri 编译（debug）
# 或一步出安装包：
pnpm tauri build                 # 前端 + Rust + 打包 NSIS .exe
```
SSH 无界面会话**能编译/出安装包**，但不能可视化跑 GUI（`tauri dev` 需桌面）；GUI 手测在真实桌面跑安装包。

## 打包目标 / 已知坑（2026-06-23）

- **打包目标 = NSIS（`.exe`）**：`tauri.conf.json` 的 `bundle.targets` 设为 `["nsis"]`。产物：`src-tauri/target/release/bundle/nsis/Lepro Connect_<ver>_x64-setup.exe`。
- **MSI 暂不出**：用 `targets:"all"` 时 WiX 的 `light.exe` 在本构建机失败（candle 通过、light 失败，疑缺 .NET Framework 3.5 或自定义 `wix/per-user-main.wxs` 的 ICE 校验）。需要 .msi 时再单独修 WiX/.NET，故默认只出 NSIS。
- **pnpm 预检坑**：pnpm（v10+）跑脚本前的 `verify-deps-before-run` 会触发 `pnpm install`，遇 `ERR_PNPM_IGNORED_BUILDS`(esbuild/msw 构建脚本未批准)直接退 1。构建脚本里设 `set PNPM_CONFIG_VERIFY_DEPS_BEFORE_RUN=false` 跳过该预检（node_modules 已装好,无需再 install）。
- **构建脚本用 ASCII**：`.cmd`/`.ps1` 里含中文注释经 GBK 控制台/PowerShell(无 BOM 当 GBK 解)会乱码、解析失败,构建/验证脚本一律纯 ASCII。
- **WebView2Loader.dll 必须随包(GNU 关键坑)**：MSVC 版静态链接 WebView2 loader,**GNU 版是动态依赖 `WebView2Loader.dll`**(`objdump -p cc-switch.exe` 可见它是唯一非系统/CRT 导入)。Tauri NSIS 默认不把它打进安装包 → 装到干净机器上启动即弹「`cc-switch.exe` 系统错误:找不到 `WebView2Loader.dll`」,窗口创建失败(进程在但无窗口)。修复:`tauri.conf.json` 加 `bundle.resources: { "target/release/WebView2Loader.dll": "WebView2Loader.dll" }`,把它装到 exe 同目录。该路径在 `tauri build` 编译后、打包前已存在,故可被解析。

## 慢网中转（GitHub/大文件下载慢时）

构建机外网（尤其 GitHub CDN）慢，但公司服务器 `192.168.33.13` 外网快、且内网可达。模式：
```
# 在 33.13 上下载（快）
ssh pangtiankai@192.168.33.13 'curl -fL -o /tmp/X <url>'
# 经本机内网中转到 Windows（scp -3 走两段 LAN）
scp -3 pangtiankai@192.168.33.13:/tmp/X administrator@<win>:C:/path/X
```
w64devkit 就是这样投递的。`cargo`/`pnpm` 若慢，可同理设国内镜像（npmmirror / crates 镜像）。

## 其它平台

本文针对 Windows 目标。macOS 安装包需在 Mac 上用 Apple 工具链 + 签名/公证，另行处理（调整 `rust-toolchain.toml` channel）。
